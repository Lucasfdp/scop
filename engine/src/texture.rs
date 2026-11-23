//! Textures (§ 14): pictures on surfaces.
//!
//! Each texture = an `R8G8B8A8_SRGB` image in device-local memory (filled through a
//! staging buffer), a view, and its own descriptor set (set 1) pointing at view +
//! the shared sampler. Texture 0 is always a 1×1 white picture, so every draw has
//! *some* texture bound.
//!
//! § 23: a texture can also be a **cube map** — six square faces in one image (6
//! layers, cube-compatible, viewed as CUBE), looked up with a direction. Texture 1 is
//! always a 1×1 white cube: the environment until the app sets one.

use ash::{Device, vk};

use crate::buffer::{Uploader, find_memory_type};
use crate::gpu::GpuError;
use crate::tga::Image;

/// How many textures the descriptor pool has room for. Plenty for scop.
const MAX_TEXTURES: u32 = 64;

/// The picture's pixel format: 8 bits each of R, G, B, A, stored **sRGB**-encoded like
/// the file, so the GPU decodes on read and the sRGB swapchain encodes on write (Q-14.5).
const FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

/// Every layer of the image (1 for a picture, 6 for a cube), mip level 0.
fn colour_range(layers: u32) -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: layers,
    }
}

/// Index of the always-present white cube (§ 23): the default environment.
pub(crate) const WHITE_CUBE: usize = 1;

/// Checks six cube faces (§ 23): all square, all the same size. Returns that size.
pub(crate) fn check_cube_faces(faces: &[&Image; 6]) -> Result<u32, GpuError> {
    let size = faces[0].width;
    let ok = size > 0 && faces.iter().all(|f| f.width == size && f.height == size);
    if ok {
        Ok(size)
    } else {
        Err(GpuError::Unsupported(
            "cube map faces must be square and all the same size",
        ))
    }
}

struct Texture {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
    /// § 23: a cube map (6 faces, sampled with a direction) rather than a picture.
    cube: bool,
    /// Set 1 for draws using this texture. Written once, freed with the pool.
    set: vk::DescriptorSet,
}

/// Every texture, the sampler they share, and the pool their sets come from.
pub(crate) struct Textures {
    sampler: vk::Sampler,
    pool: vk::DescriptorPool,
    textures: Vec<Texture>,
}

impl Textures {
    /// The shared sampler and pool, plus texture 0 (white).
    pub(crate) fn new(
        uploader: &Uploader,
        set_layout: vk::DescriptorSetLayout,
    ) -> Result<Self, GpuError> {
        let device = uploader.device;
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR) // up close: blend the 4 nearest texels
            .min_filter(vk::Filter::LINEAR) // far away: same (no mipmaps in scop)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::REPEAT) // outside 0..1: tile
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .max_lod(0.0); // only level 0 exists
        let pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(MAX_TEXTURES)];

        // SAFETY: plain creation on a live device; undone by hand until `textures` exists.
        let mut textures = unsafe {
            let sampler = device.create_sampler(&sampler_info, None)?;
            let pool = match device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .max_sets(MAX_TEXTURES)
                    .pool_sizes(&pool_sizes),
                None,
            ) {
                Ok(p) => p,
                Err(e) => {
                    device.destroy_sampler(sampler, None);
                    return Err(e.into());
                }
            };
            Textures {
                sampler,
                pool,
                textures: Vec::new(),
            }
        };

        let white = Image {
            width: 1,
            height: 1,
            rgba: vec![255; 4],
        };
        // Texture 0: white picture. Texture 1 (§ 23): white cube, the default environment.
        let added = textures
            .add(uploader, set_layout, &white)
            .and_then(|_| textures.add_cube(uploader, set_layout, [&white; 6]));
        if let Err(e) = added {
            // SAFETY: nothing submitted uses these yet (the upload waited or failed).
            unsafe { textures.destroy(device) };
            return Err(e);
        }
        Ok(textures)
    }

    /// § 23: uploads six faces as one cube map and returns its index. Face order is
    /// Vulkan's: +x, −x, +y, −y, +z, −z.
    pub(crate) fn add_cube(
        &mut self,
        uploader: &Uploader,
        set_layout: vk::DescriptorSetLayout,
        faces: [&Image; 6],
    ) -> Result<usize, GpuError> {
        check_cube_faces(&faces)?;
        if self.textures.len() >= MAX_TEXTURES as usize {
            return Err(GpuError::Unsupported("too many textures"));
        }
        let texture = upload(uploader, &faces, self.sampler, self.pool, set_layout)?;
        self.textures.push(texture);
        Ok(self.textures.len() - 1)
    }

    /// § 23: what the environment binding needs — the cube's view and the sampler.
    /// `None` if `index` isn't a cube map.
    /// The sampler every texture shares (§ 25: the probe's cube reads with it too).
    pub(crate) fn sampler(&self) -> vk::Sampler {
        self.sampler
    }

    pub(crate) fn cube(&self, index: usize) -> Option<(vk::ImageView, vk::Sampler)> {
        self.textures
            .get(index)
            .filter(|t| t.cube)
            .map(|t| (t.view, self.sampler))
    }

    /// Uploads `image` and returns its index (the number inside a `TextureId`).
    pub(crate) fn add(
        &mut self,
        uploader: &Uploader,
        set_layout: vk::DescriptorSetLayout,
        image: &Image,
    ) -> Result<usize, GpuError> {
        if self.textures.len() >= MAX_TEXTURES as usize {
            return Err(GpuError::Unsupported("too many textures"));
        }
        let texture = upload(uploader, &[image], self.sampler, self.pool, set_layout)?;
        self.textures.push(texture);
        Ok(self.textures.len() - 1)
    }

    /// The descriptor set for texture `index`, if it exists.
    pub(crate) fn set(&self, index: usize) -> Option<vk::DescriptorSet> {
        self.textures.get(index).map(|t| t.set)
    }

    /// Destroys every texture, then the pool (freeing their sets) and the sampler.
    ///
    /// # Safety
    /// Call exactly once, after the GPU has finished all work (after `device_wait_idle`).
    pub(crate) unsafe fn destroy(&self, device: &Device) {
        unsafe {
            for t in &self.textures {
                destroy_parts(device, t.image, t.memory, t.view);
            }
            device.destroy_descriptor_pool(self.pool, None);
            device.destroy_sampler(self.sampler, None);
        }
    }
}

/// File → GPU: image + memory, staging copy with the two layout transitions, view,
/// and a descriptor set written once to point at view + sampler.
/// One layer = a picture; six layers (§ 23) = a cube map, faces in Vulkan's order.
fn upload(
    uploader: &Uploader,
    layers: &[&Image],
    sampler: vk::Sampler,
    pool: vk::DescriptorPool,
    set_layout: vk::DescriptorSetLayout,
) -> Result<Texture, GpuError> {
    let device = uploader.device;
    let picture = layers[0]; // every layer has its size (checked for cubes)
    let layer_count = layers.len() as u32;
    let cube = layer_count == 6;
    let extent = vk::Extent3D {
        width: picture.width,
        height: picture.height,
        depth: 1,
    };
    let image_info = vk::ImageCreateInfo::default()
        // § 23: a cube view needs the image made "cube compatible", with 6 layers.
        .flags(if cube {
            vk::ImageCreateFlags::CUBE_COMPATIBLE
        } else {
            vk::ImageCreateFlags::empty()
        })
        .image_type(vk::ImageType::TYPE_2D)
        .format(FORMAT)
        .extent(extent)
        .mip_levels(1)
        .array_layers(layer_count)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        // Written by a copy, then read by shaders.
        .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);

    // SAFETY: plain creation on a live device; every failure below destroys what
    // exists so far. The staging buffer is destroyed after `run_once` has waited.
    unsafe {
        let image = device.create_image(&image_info, None)?;
        let needs = device.get_image_memory_requirements(image);
        let Some(type_index) = find_memory_type(
            uploader.memory_types,
            needs.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL, // written once, read every frame → fast memory
        ) else {
            device.destroy_image(image, None);
            return Err(GpuError::Unsupported("no suitable GPU memory type"));
        };
        let memory = match device.allocate_memory(
            &vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size)
                .memory_type_index(type_index),
            None,
        ) {
            Ok(m) => m,
            Err(e) => {
                device.destroy_image(image, None);
                return Err(e.into());
            }
        };
        let fail = |e: GpuError| {
            device.destroy_image(image, None);
            device.free_memory(memory, None);
            e
        };
        device
            .bind_image_memory(image, memory, 0)
            .map_err(|e| fail(e.into()))?;

        // The loading dock, then the forklift with the two layout changes (§ 14 step 3).
        // The layers one after another; the copy reads them in that order.
        let bytes: Vec<u8> = layers.iter().flat_map(|l| l.rgba.iter().copied()).collect();
        let staging = uploader.staging_with(&bytes).map_err(fail)?;
        let copied = uploader.run_once(|device, cmd| {
            record_copy(device, cmd, staging.handle, image, extent, layer_count)
        });
        staging.destroy(device); // run_once waited: the copy is done (or never ran)
        copied.map_err(fail)?;

        let view = device
            .create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(if cube {
                        vk::ImageViewType::CUBE
                    } else {
                        vk::ImageViewType::TYPE_2D
                    })
                    .format(FORMAT)
                    .subresource_range(colour_range(layer_count)),
                None,
            )
            .map_err(|e| fail(e.into()))?;

        let set_layouts = [set_layout];
        let set = match device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(pool)
                .set_layouts(&set_layouts),
        ) {
            Ok(sets) => sets[0],
            Err(e) => {
                destroy_parts(device, image, memory, view);
                return Err(e.into());
            }
        };
        // Descriptor write, once: "binding 0 = this view, read with this sampler,
        // in the layout the copy left it in".
        let image_info = [vk::DescriptorImageInfo::default()
            .sampler(sampler)
            .image_view(view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
        let write = vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_info);
        device.update_descriptor_sets(&[write], &[]);

        Ok(Texture {
            image,
            memory,
            view,
            cube,
            set,
        })
    }
}

/// barrier (ready to be written) → copy → barrier (ready to be read by shaders).
fn record_copy(
    device: &Device,
    cmd: vk::CommandBuffer,
    staging: vk::Buffer,
    image: vk::Image,
    extent: vk::Extent3D,
    layers: u32,
) {
    let to_transfer = vk::ImageMemoryBarrier::default()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(colour_range(layers));
    let to_shader = vk::ImageMemoryBarrier::default()
        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
        .dst_access_mask(vk::AccessFlags::SHADER_READ)
        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(colour_range(layers));
    // The whole picture: rows packed tightly (0 = "same as the image's width/height").
    let region = vk::BufferImageCopy {
        buffer_offset: 0,
        buffer_row_length: 0,
        buffer_image_height: 0,
        image_subresource: vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: layers, // § 23: all six faces in one copy, one after another
        },
        image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
        image_extent: extent,
    };
    // SAFETY: `cmd` is recording (inside run_once); buffer and image are alive.
    unsafe {
        device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[to_transfer],
        );
        device.cmd_copy_buffer_to_image(
            cmd,
            staging,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[region],
        );
        device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER, // where the picture is read
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[to_shader],
        );
    }
}

/// View, then image, then memory: children first.
///
/// # Safety
/// The GPU must not be using them.
unsafe fn destroy_parts(
    device: &Device,
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
) {
    unsafe {
        device.destroy_image_view(view, None);
        device.destroy_image(image, None);
        device.free_memory(memory, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(size: u32) -> Image {
        Image {
            width: size,
            height: size,
            rgba: vec![0; (size * size * 4) as usize],
        }
    }

    #[test]
    fn cube_faces_must_be_square_and_equal() {
        let (a, b) = (square(4), square(8));
        assert_eq!(check_cube_faces(&[&a; 6]).ok(), Some(4));
        assert!(check_cube_faces(&[&a, &a, &a, &a, &a, &b]).is_err()); // sizes differ
        let wide = Image {
            width: 8,
            height: 4,
            rgba: vec![0; 128],
        };
        assert!(check_cube_faces(&[&wide; 6]).is_err()); // not square
    }
}
