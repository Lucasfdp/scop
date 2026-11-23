//! Per-frame-slot objects (what the CPU reuses) and recording a frame's commands.
//!
//! A *frame slot* is a reusable kit: one command buffer (the instruction sheet),
//! one fence (the desk light), one `image_free` semaphore. Slots are used in turn.
//! Per-*image* objects (`render_done`) live with the swapchain instead.

use ash::{Device, khr, vk};

use crate::depth::DepthImage;
use crate::gpu::GpuError;
use crate::mesh::Mesh;
use crate::shader_inputs::{PUSH_SIZE, PUSH_STAGES};
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

    /// Which slot this frame uses (0 or 1): per-slot resources elsewhere use the same number.
    pub(crate) fn index(&self) -> usize {
        self.current
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
pub(crate) struct FrameTarget<'a> {
    pub(crate) image: vk::Image,
    pub(crate) view: vk::ImageView,
    pub(crate) extent: vk::Extent2D,
    /// The depth buffer (§ 11): one image, shared by every frame.
    pub(crate) depth: &'a DepthImage,
}

/// One draw, resolved from the app's tickets: real objects plus the model matrix
/// and params as the 80 bytes the push constant carries.
#[derive(Clone, Copy)]
pub(crate) struct Draw<'a> {
    pub(crate) pipeline: vk::Pipeline,
    pub(crate) mesh: &'a Mesh,
    /// Set 1: the picture this draw reads (§ 14).
    pub(crate) texture_set: vk::DescriptorSet,
    /// The sticky note: model matrix + the app's four params (§ 15, § 19), as bytes.
    pub(crate) push: [u8; PUSH_SIZE],
    /// § 22: (first instance, how many copies). Ordinary draws: (0, 1).
    pub(crate) instances: (u32, u32),
}

/// Everything a frame's commands need besides the target image.
pub(crate) struct DrawList<'a> {
    pub(crate) clear_colour: [f32; 4],
    /// The shared pipeline layout (`ShaderInputs`, § 10).
    pub(crate) layout: vk::PipelineLayout,
    /// This slot's descriptor set: points the shader at this slot's camera buffer.
    pub(crate) camera_set: vk::DescriptorSet,
    pub(crate) draws: Vec<Draw<'a>>,
}

/// § 25: the reflection probe's six views, drawn before the main pass.
pub(crate) struct ProbePass<'a> {
    pub(crate) image: vk::Image,
    /// One view per face, to draw into (+x, −x, +y, −y, +z, −z).
    pub(crate) face_views: [vk::ImageView; 6],
    pub(crate) extent: vk::Extent2D,
    pub(crate) depth: &'a DepthImage,
    /// Set 0 of each face: that face's camera, and the *fixed* environment (the probe
    /// can't read the cube it's drawing into).
    pub(crate) sets: [vk::DescriptorSet; 6],
    /// The draws that appear in reflections (not the reflective model, not text).
    pub(crate) draws: Vec<Draw<'a>>,
}

/// The probe image's six layers.
const PROBE_LAYERS: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    base_mip_level: 0,
    level_count: 1,
    base_array_layer: 0,
    layer_count: 6,
};

/// Records one frame: (§ 25: the probe's six views first, if any), then make the
/// image drawable, clear it, bind the camera, draw each mesh with its own model matrix
/// (dynamic rendering), then make it presentable.
pub(crate) fn record_frame(
    device: &Device,
    rendering: &khr::dynamic_rendering::Device,
    command_buffer: vk::CommandBuffer,
    target: &FrameTarget,
    list: &DrawList,
    probe: Option<&ProbePass>,
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
    // § 11: the depth image, ready for depth testing. Old layout UNDEFINED: it's
    // cleared below anyway. Waits for the *previous* frame's depth writes (same
    // image, frames in flight) before this frame's depth tests read or write it.
    let depth_ready = vk::ImageMemoryBarrier::default()
        .src_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
        .dst_access_mask(
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        )
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(target.depth.image)
        .subresource_range(target.depth.range());
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
            color: vk::ClearColorValue {
                float32: list.clear_colour,
            },
        })];
    // § 11: start every pixel at 1.0 (far plane), so the first fragment anywhere wins;
    // throw the numbers away afterwards (DONT_CARE): nothing reads them after the frame.
    let depth_attachment = vk::RenderingAttachmentInfo::default()
        .image_view(target.depth.view)
        .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .clear_value(vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        });
    let rendering_info = vk::RenderingInfo::default()
        .render_area(whole_image)
        .layer_count(1)
        .color_attachments(&colour_attachments)
        .depth_attachment(&depth_attachment);
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
        // § 25: the probe first, so its cube is finished before the main pass reads it.
        if let Some(probe) = probe {
            record_probe(device, rendering, command_buffer, list, probe);
        }
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
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::LATE_FRAGMENT_TESTS, // where last frame's depth writes end
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS, // where this frame's begin
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[depth_ready],
        );
        rendering.cmd_begin_rendering(command_buffer, &rendering_info);
        // Dynamic state, set once: it stays set across the pipeline binds below.
        device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        device.cmd_set_scissor(command_buffer, 0, &[whole_image]);
        // The noticeboard: set slot 0 = this slot's camera. Bound once — every pipeline
        // shares the layout, so it stays bound across the pipeline switches below.
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            list.layout,
            0,                  // first set slot
            &[list.camera_set], // the sets, from slot 0 on
            &[],                // no dynamic offsets
        );
        record_draws(device, command_buffer, list.layout, &list.draws);
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

/// Every draw of a list, in order: pipeline, picture, sticky note, mesh, draw. Set 0
/// (the camera) must already be bound.
///
/// # Safety
/// Inside a rendering pass on `command_buffer`, with everything in `draws` alive.
unsafe fn record_draws(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    layout: vk::PipelineLayout,
    draws: &[Draw],
) {
    // SAFETY: guaranteed by the caller.
    unsafe {
        for draw in draws {
            let mesh = draw.mesh;
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                draw.pipeline,
            );
            // Set slot 1 = this draw's texture (the camera in slot 0 stays bound).
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                layout,
                1,
                &[draw.texture_set],
                &[],
            );
            // The sticky note: model (bytes 0..64) + params (64..80), both stages —
            // the same stages and size as the layout's range, or validation complains.
            device.cmd_push_constants(command_buffer, layout, PUSH_STAGES, 0, &draw.push);
            // Corners from binding 0, triangles from the index buffer (32-bit indices).
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[mesh.vertices.handle], &[0]);
            device.cmd_bind_index_buffer(
                command_buffer,
                mesh.indices.handle,
                0,
                vk::IndexType::UINT32,
            );
            // All indices, from index 0, no vertex offset; § 22: `count` copies whose
            // matrices start at `first` in the instance list (gl_InstanceIndex counts
            // from there). Ordinary draws: 1 copy at position 0, the identity.
            let (first, count) = draw.instances;
            device.cmd_draw_indexed(command_buffer, mesh.index_count, count, 0, 0, first);
        }
    }
}

/// § 25: the probe's six views. Hand-over in (any → "drawn into"), one pass per face
/// with that face's camera, hand-over out ("drawn into" → "read by shaders", Q-25.4).
///
/// # Safety
/// Recording on `command_buffer`, outside any rendering pass, everything alive.
unsafe fn record_probe(
    device: &Device,
    rendering: &khr::dynamic_rendering::Device,
    command_buffer: vk::CommandBuffer,
    list: &DrawList,
    probe: &ProbePass,
) {
    let layout_change = |old, new, src_access, dst_access| {
        vk::ImageMemoryBarrier::default()
            .src_access_mask(src_access)
            .dst_access_mask(dst_access)
            .old_layout(old)
            .new_layout(new)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(probe.image)
            .subresource_range(PROBE_LAYERS)
    };
    // Old layout UNDEFINED: every face is cleared anyway. Waits for last frame's
    // chrome to finish reading the cube before drawing into it again.
    let to_drawable = layout_change(
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::AccessFlags::SHADER_READ,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
    );
    let to_readable = layout_change(
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        vk::AccessFlags::SHADER_READ,
    );
    // The probe's depth image is reused by every face: each face waits for the last
    // one's depth writes (the same rule as § 11's frames in flight).
    let depth_ready = vk::ImageMemoryBarrier::default()
        .src_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
        .dst_access_mask(
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        )
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(probe.depth.image)
        .subresource_range(probe.depth.range());
    let area = vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: probe.extent,
    };
    let viewport = vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: probe.extent.width as f32,
        height: probe.extent.height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    };

    // SAFETY: guaranteed by the caller.
    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[to_drawable],
        );
        for (face, &view) in probe.face_views.iter().enumerate() {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                    | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[depth_ready],
            );
            let colour = [vk::RenderingAttachmentInfo::default()
                .image_view(view)
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE) // kept: chrome reads it next
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: list.clear_colour,
                    },
                })];
            let depth = vk::RenderingAttachmentInfo::default()
                .image_view(probe.depth.view)
                .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .clear_value(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                });
            let info = vk::RenderingInfo::default()
                .render_area(area)
                .layer_count(1)
                .color_attachments(&colour)
                .depth_attachment(&depth);
            rendering.cmd_begin_rendering(command_buffer, &info);
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[area]);
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                list.layout,
                0,
                &[probe.sets[face]], // this face's camera
                &[],
            );
            record_draws(device, command_buffer, list.layout, &probe.draws);
            rendering.cmd_end_rendering(command_buffer);
        }
        // Hand-over: finished drawing → shaders may read it (Q-25.4).
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[to_readable],
        );
    }
}
