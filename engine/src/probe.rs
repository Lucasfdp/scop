//! The reflection probe (§ 25): a cube map the engine draws the scene into, six
//! views a frame, from one point — so chrome reflects whatever is really around.
//!
//! - One image, 6 layers, "cube compatible": drawn into one layer at a time (a 2D view
//!   per layer), read as a cube (one cube view) — render to texture.
//! - Its own depth image, reused (and cleared) for each face.
//! - Same colour and depth formats as the screen, so every pipeline can draw into it.

use ash::{Device, Instance, vk};

use crate::buffer::find_memory_type;
use crate::depth::DepthImage;
use crate::gpu::GpuError;
use crate::maths::{Mat4, Vec3};

/// Texels per face side: reflections are small and curved on screen (Q-25.5).
pub(crate) const PROBE_SIZE: u32 = 256;

pub(crate) struct ReflectionProbe {
    pub(crate) image: vk::Image,
    memory: vk::DeviceMemory,
    /// One 2D view per face, to draw into (Vulkan's order: +x, −x, +y, −y, +z, −z).
    pub(crate) face_views: [vk::ImageView; 6],
    /// The whole image as a cube, to read from (the environment, set 0 binding 2).
    pub(crate) cube_view: vk::ImageView,
    pub(crate) depth: DepthImage,
}

impl ReflectionProbe {
    /// Creates the cube render target in `format` (the swapchain's) and its depth image.
    pub(crate) fn new(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        device: &Device,
        memory_types: &vk::PhysicalDeviceMemoryProperties,
        format: vk::Format,
    ) -> Result<Self, GpuError> {
        let extent = vk::Extent2D {
            width: PROBE_SIZE,
            height: PROBE_SIZE,
        };
        let depth = DepthImage::new(instance, physical_device, device, memory_types, extent)?;
        let image_info = vk::ImageCreateInfo::default()
            .flags(vk::ImageCreateFlags::CUBE_COMPATIBLE)
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: PROBE_SIZE,
                height: PROBE_SIZE,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(6)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            // Drawn into (render target) *and* read by shaders (a texture).
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        // SAFETY: plain creation on a live device; each failure undoes what exists.
        unsafe {
            let fail = |e: GpuError| {
                depth.destroy(device);
                e
            };
            let image = device
                .create_image(&image_info, None)
                .map_err(|e| fail(e.into()))?;
            let needs = device.get_image_memory_requirements(image);
            let Some(type_index) = find_memory_type(
                memory_types,
                needs.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ) else {
                device.destroy_image(image, None);
                return Err(fail(GpuError::Unsupported("no suitable GPU memory type")));
            };
            let alloc = vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size)
                .memory_type_index(type_index);
            let memory = match device.allocate_memory(&alloc, None) {
                Ok(m) => m,
                Err(e) => {
                    device.destroy_image(image, None);
                    return Err(fail(e.into()));
                }
            };
            let undo = |views: &[vk::ImageView]| {
                for &v in views {
                    device.destroy_image_view(v, None);
                }
                device.destroy_image(image, None);
                device.free_memory(memory, None);
                depth.destroy(device);
            };
            if let Err(e) = device.bind_image_memory(image, memory, 0) {
                undo(&[]);
                return Err(e.into());
            }
            let view = |kind, first_layer, layers| {
                device.create_image_view(
                    &vk::ImageViewCreateInfo::default()
                        .image(image)
                        .view_type(kind)
                        .format(format)
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: first_layer,
                            layer_count: layers,
                        }),
                    None,
                )
            };
            let mut views = Vec::with_capacity(7);
            for face in 0..6 {
                match view(vk::ImageViewType::TYPE_2D, face, 1) {
                    Ok(v) => views.push(v),
                    Err(e) => {
                        undo(&views);
                        return Err(e.into());
                    }
                }
            }
            let cube_view = match view(vk::ImageViewType::CUBE, 0, 6) {
                Ok(v) => v,
                Err(e) => {
                    undo(&views);
                    return Err(e.into());
                }
            };
            Ok(ReflectionProbe {
                image,
                memory,
                face_views: [views[0], views[1], views[2], views[3], views[4], views[5]],
                cube_view,
                depth,
            })
        }
    }

    /// # Safety
    /// Call once, after the GPU has finished with the probe.
    pub(crate) unsafe fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_image_view(self.cube_view, None);
            for &v in &self.face_views {
                device.destroy_image_view(v, None);
            }
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
            self.depth.destroy(device);
        }
    }
}

/// The six face cameras of a probe at `position` (§ 25): view-projection cards, in
/// Vulkan's face order. Each looks at one wall with a square, 90° view, oriented so
/// that what it draws lands where the cube lookup (§ 23's table) will look for it.
///
/// Unlike the screen camera, the projection here is *not* y-flipped: a cube face's
/// rows run "down" its own way (the table), not "down the screen".
pub(crate) fn face_cameras(position: Vec3, near: f32, far: f32) -> [Mat4; 6] {
    // (looks along, the face's "down", which `look_at` calls up), per Vulkan's table.
    let x = Vec3::new(1.0, 0.0, 0.0);
    let y = Vec3::new(0.0, 1.0, 0.0);
    let z = Vec3::new(0.0, 0.0, 1.0);
    let faces = [(x, -y), (-x, -y), (y, z), (-y, -z), (z, -y), (-z, -y)];
    // The screen's perspective card flips y (§ 9); flip it back.
    let projection = Mat4::scale(Vec3::new(1.0, -1.0, 1.0))
        * Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, near, far);
    faces.map(|(forward, down)| projection * Mat4::look_at(position, position + forward, down))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maths::Vec4;
    use crate::sky::texel_direction;

    #[test]
    fn each_face_draws_where_the_cube_lookup_reads() {
        // For texel (s, t) of face f, the GPU reads in direction `texel_direction` (§ 23,
        // checked against Vulkan's table). The face camera must draw that direction at
        // exactly that spot of its image: NDC (2s − 1, 2t − 1).
        let probe = Vec3::new(3.0, -1.0, 2.0);
        let cameras = face_cameras(probe, 0.01, 100.0);
        for (face, camera) in cameras.iter().enumerate() {
            for &(s, t) in &[(0.5, 0.5), (0.2, 0.7), (0.9, 0.1), (0.3, 0.35)] {
                let point = probe + texel_direction(face, s, t) * 5.0;
                let clip = *camera * Vec4::point(point);
                let (ndc_x, ndc_y) = (clip.x / clip.w, clip.y / clip.w);
                assert!(
                    (ndc_x - (2.0 * s - 1.0)).abs() < 1e-4
                        && (ndc_y - (2.0 * t - 1.0)).abs() < 1e-4,
                    "face {face} at ({s}, {t}) drew at ({ndc_x}, {ndc_y})"
                );
                assert!(clip.w > 0.0, "face {face}: the point is behind the camera");
            }
        }
    }

    #[test]
    fn six_faces_see_every_direction_once() {
        // A spread of directions: each lands inside exactly one face's view.
        let cameras = face_cameras(Vec3::ZERO, 0.01, 100.0);
        for i in 0..200 {
            let a = i as f32 * 0.37;
            let b = i as f32 * 0.91;
            let d = Vec3::new(a.cos() * b.sin(), b.cos(), a.sin() * b.sin()) * 3.0;
            let seen_by = cameras
                .iter()
                .filter(|c| {
                    let clip = **c * Vec4::point(d);
                    let inside = |v: f32| v.abs() < clip.w;
                    clip.w > 0.0 && inside(clip.x) && inside(clip.y)
                })
                .count();
            assert_eq!(seen_by, 1, "direction {d:?}");
        }
    }
}
