//! The depth buffer (§ 11): one swapchain-sized image holding, for every pixel, how
//! far away the thing currently drawn there is. The GPU tests and updates it itself;
//! the engine only creates it, clears it each frame, and destroys it.
//!
//! Rebuilt with the swapchain on resize (§ 16).

use ash::{Device, Instance, vk};

use crate::buffer::find_memory_type;
use crate::gpu::GpuError;

/// Depth formats scop accepts, best first:
/// `D32_SFLOAT` — Depth, 32-bit Signed FLOAT (no stencil; the usual pick, and what
/// MoltenVK offers); the other two carry a stencil part scop doesn't use.
const CANDIDATES: [vk::Format; 3] = [
    vk::Format::D32_SFLOAT,
    vk::Format::D32_SFLOAT_S8_UINT,
    vk::Format::D24_UNORM_S8_UINT,
];

/// A depth image, its memory and its view, owned and destroyed together.
pub(crate) struct DepthImage {
    pub(crate) image: vk::Image,
    memory: vk::DeviceMemory,
    pub(crate) view: vk::ImageView,
    pub(crate) format: vk::Format,
    /// `DEPTH`, plus `STENCIL` if the format has a stencil part: views and barriers
    /// must name every part of a combined depth/stencil image.
    pub(crate) aspect: vk::ImageAspectFlags,
}

impl DepthImage {
    /// Picks a supported depth format, then creates an image of `extent` in
    /// device-local memory and a view of it (same three steps as § 7's `Buffer`, plus a view).
    pub(crate) fn new(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        device: &Device,
        memory_types: &vk::PhysicalDeviceMemoryProperties,
        extent: vk::Extent2D,
    ) -> Result<Self, GpuError> {
        let format = first_supported(&CANDIDATES, |f| {
            // SAFETY: plain query on a live instance.
            unsafe { instance.get_physical_device_format_properties(physical_device, f) }
                .optimal_tiling_features
        })
        .ok_or(GpuError::Unsupported("no usable depth format"))?;
        let aspect = aspect_of(format);

        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1, // a flat 2D image (this "depth" is the 3rd dimension, unrelated to § 11)
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL) // GPU's own layout; the CPU never reads it
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        // SAFETY: plain creation on a live device; each failure undoes what exists.
        unsafe {
            let image = device.create_image(&image_info, None)?;

            let needs = device.get_image_memory_requirements(image);
            let Some(type_index) = find_memory_type(
                memory_types,
                needs.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL, // only the GPU ever touches it
            ) else {
                device.destroy_image(image, None);
                return Err(GpuError::Unsupported("no suitable GPU memory type"));
            };
            let alloc = vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size)
                .memory_type_index(type_index);
            let memory = match device.allocate_memory(&alloc, None) {
                Ok(m) => m,
                Err(e) => {
                    device.destroy_image(image, None);
                    return Err(e.into());
                }
            };
            let free_both = || {
                device.destroy_image(image, None);
                device.free_memory(memory, None);
            };
            if let Err(e) = device.bind_image_memory(image, memory, 0) {
                free_both();
                return Err(e.into());
            }

            let view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(range(aspect));
            let view = match device.create_image_view(&view_info, None) {
                Ok(v) => v,
                Err(e) => {
                    free_both();
                    return Err(e.into());
                }
            };

            Ok(DepthImage {
                image,
                memory,
                view,
                format,
                aspect,
            })
        }
    }

    /// The whole image, for barriers: all its aspects, mip 0, layer 0.
    pub(crate) fn range(&self) -> vk::ImageSubresourceRange {
        range(self.aspect)
    }

    /// Destroys the view, then the image, then frees its memory.
    ///
    /// # Safety
    /// Call exactly once, after the GPU has finished all work using the image
    /// (after `device_wait_idle`), before the device is destroyed.
    pub(crate) unsafe fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }
}

/// The first format in `candidates` usable as a depth attachment, according to
/// `features_of` (its optimal-tiling features).
fn first_supported(
    candidates: &[vk::Format],
    features_of: impl Fn(vk::Format) -> vk::FormatFeatureFlags,
) -> Option<vk::Format> {
    candidates
        .iter()
        .copied()
        .find(|&f| features_of(f).contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT))
}

/// `DEPTH`, plus `STENCIL` for formats that have a stencil part (`…_S8_UINT`).
fn aspect_of(format: vk::Format) -> vk::ImageAspectFlags {
    match format {
        vk::Format::D32_SFLOAT_S8_UINT | vk::Format::D24_UNORM_S8_UINT => {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
        }
        _ => vk::ImageAspectFlags::DEPTH,
    }
}

fn range(aspect: vk::ImageAspectFlags) -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask: aspect,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_first_format_with_depth_support() {
        // Pretend only the third candidate can be a depth attachment.
        let pick = first_supported(&CANDIDATES, |f| {
            if f == vk::Format::D24_UNORM_S8_UINT {
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT
            } else {
                vk::FormatFeatureFlags::empty()
            }
        });
        assert_eq!(pick, Some(vk::Format::D24_UNORM_S8_UINT));
        // And the best one wins when everything is supported.
        let all = first_supported(&CANDIDATES, |_| {
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT
        });
        assert_eq!(all, Some(vk::Format::D32_SFLOAT));
        // Nothing supported → None (turned into a clear error).
        assert_eq!(
            first_supported(&CANDIDATES, |_| vk::FormatFeatureFlags::empty()),
            None
        );
    }

    #[test]
    fn stencil_formats_name_both_aspects() {
        assert_eq!(
            aspect_of(vk::Format::D32_SFLOAT),
            vk::ImageAspectFlags::DEPTH
        );
        assert!(aspect_of(vk::Format::D24_UNORM_S8_UINT).contains(vk::ImageAspectFlags::STENCIL));
    }
}
