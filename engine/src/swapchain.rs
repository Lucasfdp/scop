//! The swapchain: the queue of images Vulkan draws into and the screen displays,
//! plus one image view per image.
//!
//! Owned by `GpuContext`, which calls `destroy` before destroying the device.

use ash::{Device, khr, vk};

use crate::gpu::GpuError;

pub(crate) struct Swapchain {
    pub(crate) handle: vk::SwapchainKHR,
    pub(crate) images: Vec<vk::Image>, // owned by the swapchain, never destroyed by us
    pub(crate) views: Vec<vk::ImageView>,
    pub(crate) format: vk::SurfaceFormatKHR,
    pub(crate) present_mode: vk::PresentModeKHR,
    pub(crate) extent: vk::Extent2D,
}

impl Swapchain {
    /// Asks the surface what it supports, picks from it, and creates the swapchain
    /// and its image views. `window_size` is in physical pixels.
    pub(crate) fn new(
        device: &Device,
        loader: &khr::swapchain::Device,
        surface_loader: &khr::surface::Instance,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
        window_size: (u32, u32),
    ) -> Result<Self, GpuError> {
        // The three menus the surface offers.
        // SAFETY: plain queries on a live physical device and surface.
        let (caps, formats, present_modes) = unsafe {
            (
                surface_loader
                    .get_physical_device_surface_capabilities(physical_device, surface)?,
                surface_loader.get_physical_device_surface_formats(physical_device, surface)?,
                surface_loader
                    .get_physical_device_surface_present_modes(physical_device, surface)?,
            )
        };

        let format = choose_format(&formats);
        let present_mode = choose_present_mode(&present_modes);
        let extent = choose_extent(&caps, window_size);
        let image_count = choose_image_count(&caps);

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(choose_composite_alpha(&caps))
            .present_mode(present_mode)
            .clipped(true);

        // SAFETY: every pointer in `create_info` points to data alive until this call returns.
        let handle = unsafe { loader.create_swapchain(&create_info, None)? };

        // SAFETY: `handle` was just created from this loader.
        let images = match unsafe { loader.get_swapchain_images(handle) } {
            Ok(images) => images,
            Err(e) => {
                unsafe { loader.destroy_swapchain(handle, None) };
                return Err(e.into());
            }
        };

        let mut views = Vec::with_capacity(images.len());
        for &image in &images {
            match create_view(device, image, format.format) {
                Ok(view) => views.push(view),
                Err(e) => {
                    // Undo what this function made so far: views, then the swapchain.
                    // SAFETY: none of these are in use yet.
                    unsafe {
                        for &view in &views {
                            device.destroy_image_view(view, None);
                        }
                        loader.destroy_swapchain(handle, None);
                    }
                    return Err(e);
                }
            }
        }

        Ok(Swapchain {
            handle,
            images,
            views,
            format,
            present_mode,
            extent,
        })
    }

    /// Destroys the views, then the swapchain (which frees its own images).
    ///
    /// # Safety
    /// Call exactly once, before the device is destroyed, when the GPU is no longer
    /// using any of the images.
    pub(crate) unsafe fn destroy(&self, device: &Device, loader: &khr::swapchain::Device) {
        unsafe {
            for &view in &self.views {
                device.destroy_image_view(view, None);
            }
            loader.destroy_swapchain(self.handle, None);
        }
    }
}

/// 8-bit BGRA in sRGB if offered (correct brightness for free), else whatever comes first.
fn choose_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    formats
        .iter()
        .copied()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or(formats[0])
}

/// FIFO: wait for the screen's refresh (vsync). The spec guarantees every driver has it.
fn choose_present_mode(_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    vk::PresentModeKHR::FIFO
}

/// The surface's own size, unless it says "you choose" (width = u32::MAX):
/// then the window's physical size, clamped to what the surface allows.
fn choose_extent(caps: &vk::SurfaceCapabilitiesKHR, window_size: (u32, u32)) -> vk::Extent2D {
    if caps.current_extent.width != u32::MAX {
        return caps.current_extent;
    }
    vk::Extent2D {
        width: window_size
            .0
            .clamp(caps.min_image_extent.width, caps.max_image_extent.width),
        height: window_size
            .1
            .clamp(caps.min_image_extent.height, caps.max_image_extent.height),
    }
}

/// One more than the minimum, so there's a spare image; clamp only if there is a
/// maximum (0 means "no limit").
fn choose_image_count(caps: &vk::SurfaceCapabilitiesKHR) -> u32 {
    let wanted = caps.min_image_count + 1;
    if caps.max_image_count > 0 {
        wanted.min(caps.max_image_count)
    } else {
        wanted
    }
}

/// OPAQUE (solid window) if supported, else the first mode the surface supports.
fn choose_composite_alpha(caps: &vk::SurfaceCapabilitiesKHR) -> vk::CompositeAlphaFlagsKHR {
    [
        vk::CompositeAlphaFlagsKHR::OPAQUE,
        vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
        vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
        vk::CompositeAlphaFlagsKHR::INHERIT,
    ]
    .into_iter()
    .find(|&mode| caps.supported_composite_alpha.contains(mode))
    .unwrap_or(vk::CompositeAlphaFlagsKHR::OPAQUE)
}

/// A plain 2D colour view of the whole image: one mip level, one layer.
fn create_view(
    device: &Device,
    image: vk::Image,
    format: vk::Format,
) -> Result<vk::ImageView, GpuError> {
    let info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });
    // SAFETY: `image` belongs to a live swapchain on this device.
    Ok(unsafe { device.create_image_view(&info, None)? })
}
