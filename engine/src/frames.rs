//! Per-frame-slot objects (what the CPU reuses) and recording a frame's commands.
//!
//! A *frame slot* is a reusable kit: one command buffer (the instruction sheet),
//! one fence (the desk light), one `image_free` semaphore. Slots are used in turn.
//! Per-*image* objects (`render_done`) live with the swapchain instead.

use ash::{Device, khr, vk};

use crate::gpu::GpuError;
use crate::mesh::Mesh;
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

    /// The pool command buffers are allocated from (also used for one-off uploads).
    pub(crate) fn pool(&self) -> vk::CommandPool {
        self.pool
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

/// Everything one frame draws with. Handles only, borrowed from their owners.
pub(crate) struct FrameTarget {
    pub(crate) image: vk::Image,
    pub(crate) view: vk::ImageView,
    pub(crate) extent: vk::Extent2D,
}

/// Records one frame: make the image drawable, clear it to `colour`, draw each
/// (pipeline, mesh) pair in order with dynamic rendering, then make it presentable.
pub(crate) fn record_frame(
    device: &Device,
    rendering: &khr::dynamic_rendering::Device,
    command_buffer: vk::CommandBuffer,
    target: &FrameTarget,
    draws: &[(vk::Pipeline, &Mesh)],
    colour: [f32; 4],
) -> Result<(), GpuError> {
    let to_drawable = vk::ImageMemoryBarrier::default()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED) // "don't care what was there"
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(target.image)
        .subresource_range(COLOUR_RANGE);
    let to_present = vk::ImageMemoryBarrier::default()
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::empty())
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(target.image)
        .subresource_range(COLOUR_RANGE);

    let whole_image = vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: target.extent,
    };
    // Load op CLEAR replaces § 5's cmd_clear_color_image.
    let colour_attachments = [vk::RenderingAttachmentInfo::default()
        .image_view(target.view)
        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .clear_value(vk::ClearValue {
            color: vk::ClearColorValue { float32: colour },
        })];
    let rendering_info = vk::RenderingInfo::default()
        .render_area(whole_image)
        .layer_count(1)
        .color_attachments(&colour_attachments);
    let viewport = vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: target.extent.width as f32,
        height: target.extent.height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    };
    let begin_info =
        vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    // SAFETY: the caller waited on this slot's fence, so the GPU is no longer using
    // `command_buffer`; the image, view and pipeline are alive.
    unsafe {
        device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;
        device.begin_command_buffer(command_buffer, &begin_info)?;
        // Source stage matches the stage the submit waits on `image_free` at.
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[to_drawable],
        );
        rendering.cmd_begin_rendering(command_buffer, &rendering_info);
        // Dynamic state, set once: it stays set across the pipeline binds below.
        device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        device.cmd_set_scissor(command_buffer, 0, &[whole_image]);
        for &(pipeline, mesh) in draws {
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
            // Corners from binding 0, triangles from the index buffer (32-bit indices).
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[mesh.vertices.handle], &[0]);
            device.cmd_bind_index_buffer(
                command_buffer,
                mesh.indices.handle,
                0,
                vk::IndexType::UINT32,
            );
            // All indices, 1 instance, from index 0, no vertex offset, instance 0.
            device.cmd_draw_indexed(command_buffer, mesh.index_count, 1, 0, 0, 0);
        }
        rendering.cmd_end_rendering(command_buffer);
        // The drawing must finish before the frame counts as done.
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
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
