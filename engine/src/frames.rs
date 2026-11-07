//! Per-frame-slot objects (what the CPU reuses) and recording a frame's commands.
//!
//! A *frame slot* is a reusable kit: one command buffer (the instruction sheet),
//! one fence (the desk light), one `image_free` semaphore. Slots are used in turn.
//! Per-*image* objects (`render_done`) live with the swapchain instead.

use ash::{Device, vk};

use crate::gpu::GpuError;
use crate::swapchain::COLOUR_RANGE;

/// How many frames the CPU may work ahead of the GPU.
pub(crate) const FRAMES_IN_FLIGHT: usize = 2;

pub(crate) struct Slot {
    pub(crate) command_buffer: vk::CommandBuffer,
    pub(crate) fence: vk::Fence,
    pub(crate) image_free: vk::Semaphore,
}

pub(crate) struct Frames {
    pool: vk::CommandPool,
    slots: Vec<Slot>,
    current: usize,
}

impl Frames {
    /// Creates the command pool for `queue_family_index` and one slot per frame in flight.
    pub(crate) fn new(device: &Device, queue_family_index: u32) -> Result<Self, GpuError> {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        // SAFETY: plain creation on a live device.
        let pool = unsafe { device.create_command_pool(&pool_info, None)? };

        let mut frames = Frames {
            pool,
            slots: Vec::with_capacity(FRAMES_IN_FLIGHT),
            current: 0,
        };
        for _ in 0..FRAMES_IN_FLIGHT {
            match create_slot(device, pool) {
                Ok(slot) => frames.slots.push(slot),
                Err(e) => {
                    // SAFETY: nothing has been submitted yet, so nothing is in use.
                    unsafe { frames.destroy(device) };
                    return Err(e);
                }
            }
        }
        Ok(frames)
    }

    /// The slot this frame uses.
    pub(crate) fn current(&self) -> &Slot {
        &self.slots[self.current]
    }

    /// Moves on to the next slot: 0, 1, 0, 1…
    pub(crate) fn advance(&mut self) {
        self.current = (self.current + 1) % self.slots.len();
    }

    /// Destroys the semaphores and fences, then the pool (which frees its command buffers).
    ///
    /// # Safety
    /// Call exactly once, when the GPU has finished all work using these objects
    /// (after `device_wait_idle`), before the device is destroyed.
    pub(crate) unsafe fn destroy(&self, device: &Device) {
        unsafe {
            for slot in &self.slots {
                device.destroy_semaphore(slot.image_free, None);
                device.destroy_fence(slot.fence, None);
            }
            device.destroy_command_pool(self.pool, None);
        }
    }
}

fn create_slot(device: &Device, pool: vk::CommandPool) -> Result<Slot, GpuError> {
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    // Created already ON: the first wait on it must pass, because no work was ever
    // submitted with it (§ 5 checkpoint step 7: remove SIGNALED and watch it hang).
    let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
    let semaphore_info = vk::SemaphoreCreateInfo::default();

    // SAFETY: plain creation on a live device. The command buffer is freed with the
    // pool, so only the fence needs undoing if the semaphore fails.
    unsafe {
        let command_buffer = device.allocate_command_buffers(&alloc_info)?[0];
        let fence = device.create_fence(&fence_info, None)?;
        let image_free = match device.create_semaphore(&semaphore_info, None) {
            Ok(s) => s,
            Err(e) => {
                device.destroy_fence(fence, None);
                return Err(e.into());
            }
        };
        Ok(Slot {
            command_buffer,
            fence,
            image_free,
        })
    }
}

/// Records one frame: make `image` writable, clear it to `colour`, make it presentable.
pub(crate) fn record_clear(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    colour: [f32; 4],
) -> Result<(), GpuError> {
    let to_transfer = vk::ImageMemoryBarrier::default()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED) // "don't care what was there"
        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(COLOUR_RANGE);
    let to_present = vk::ImageMemoryBarrier::default()
        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
        .dst_access_mask(vk::AccessFlags::empty())
        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(COLOUR_RANGE);
    let clear_value = vk::ClearColorValue { float32: colour };
    let begin_info =
        vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    // SAFETY: the caller waited on this slot's fence, so the GPU is no longer
    // using `command_buffer`; `image` is a live swapchain image from acquire.
    unsafe {
        device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;
        device.begin_command_buffer(command_buffer, &begin_info)?;
        // Source stage TRANSFER matches the stage the submit waits on `image_free` at.
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[to_transfer],
        );
        device.cmd_clear_color_image(
            command_buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &clear_value,
            &[COLOUR_RANGE],
        );
        // The clear must finish before the frame counts as done.
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[to_present],
        );
        device.end_command_buffer(command_buffer)?;
    }
    Ok(())
}
