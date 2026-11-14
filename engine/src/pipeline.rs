//! The graphics pipeline: shaders + every fixed-function switch, frozen into one object.

use std::ffi::CStr;

use ash::{Device, vk};

use crate::gpu::GpuError;
use crate::mesh::Vertex;
use crate::spirv;

const ENTRY_POINT: &CStr = c"main";

/// § 11: a fragment survives if its depth is LESS than what's stored (smaller = nearer).
/// Checkpoint experiment: try `GREATER` and only the background is left (Q-11.3).
const DEPTH_COMPARE: vk::CompareOp = vk::CompareOp::LESS;

/// Compiled shaders the app hands to the engine (SPIR-V bytes, e.g. from `include_bytes!`).
#[derive(Clone, Copy)]
pub struct Shaders {
    pub vertex: &'static [u8],
    pub fragment: &'static [u8],
}

/// A pipeline. Its layout is the engine-wide one from `ShaderInputs` (§ 10),
/// shared and destroyed there, not owned here.
pub(crate) struct Pipeline {
    pub(crate) handle: vk::Pipeline,
}

impl Pipeline {
    /// Builds a pipeline that draws triangles into images of `colour_format`, with a
    /// depth image of `depth_format`, reading the extra inputs described by `layout`.
    pub(crate) fn new(
        device: &Device,
        layout: vk::PipelineLayout,
        colour_format: vk::Format,
        depth_format: vk::Format,
        shaders: Shaders,
    ) -> Result<Self, GpuError> {
        let vertex_words = spirv::words_from_bytes(shaders.vertex)?;
        let fragment_words = spirv::words_from_bytes(shaders.fragment)?;

        // SAFETY: plain creation on a live device; every object made here is
        // destroyed below on failure, or owned by the returned Pipeline.
        unsafe {
            let modules =
                create_module(device, &vertex_words).and_then(|vertex| {
                    match create_module(device, &fragment_words) {
                        Ok(fragment) => Ok((vertex, fragment)),
                        Err(e) => {
                            device.destroy_shader_module(vertex, None);
                            Err(e)
                        }
                    }
                });
            let (vertex, fragment) = modules?;

            let formats = (colour_format, depth_format);
            let result = create_pipeline(device, layout, formats, vertex, fragment);

            // The pipeline keeps its own compiled copy; the modules can go now.
            device.destroy_shader_module(vertex, None);
            device.destroy_shader_module(fragment, None);

            Ok(Pipeline { handle: result? })
        }
    }

    /// # Safety
    /// Call exactly once, after the GPU has finished all work using this pipeline,
    /// before the device is destroyed.
    pub(crate) unsafe fn destroy(&self, device: &Device) {
        unsafe { device.destroy_pipeline(self.handle, None) };
    }
}

fn create_module(device: &Device, words: &[u32]) -> Result<vk::ShaderModule, GpuError> {
    let info = vk::ShaderModuleCreateInfo::default().code(words);
    // SAFETY: `words` is validated SPIR-V (length, magic number) and outlives the call.
    Ok(unsafe { device.create_shader_module(&info, None)? })
}

/// Fills in every part of the pipeline (the table in § 6, step 5) and creates it.
fn create_pipeline(
    device: &Device,
    layout: vk::PipelineLayout,
    (colour_format, depth_format): (vk::Format, vk::Format),
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
) -> Result<vk::Pipeline, GpuError> {
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex)
            .name(ENTRY_POINT),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment)
            .name(ENTRY_POINT),
    ];

    // The packing list (§ 7): binding 0 holds `Vertex`es; locations 0 and 1 read from it.
    let bindings = [Vertex::binding()];
    let attributes = Vertex::attributes();
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&bindings)
        .vertex_attribute_descriptions(&attributes);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    // One viewport and one scissor; their values are dynamic (set while recording).
    let viewport = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    let rasterisation = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .line_width(1.0);

    let multisample = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    // § 6 checkpoint step 6: set this to `ColorComponentFlags::empty()` and the
    // triangle disappears — drawn, but nothing written.
    let blend_attachments = [vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(false)
        .color_write_mask(vk::ColorComponentFlags::RGBA)];
    let colour_blend =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(&blend_attachments);

    // § 11: test every fragment against the depth buffer, and remember the winners.
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(DEPTH_COMPARE);

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    // Dynamic rendering's replacement for a render pass: just the formats drawn into.
    // The depth format must match the depth image attached while drawing.
    let colour_formats = [colour_format];
    let mut rendering = vk::PipelineRenderingCreateInfo::default()
        .color_attachment_formats(&colour_formats)
        .depth_attachment_format(depth_format);

    let info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport)
        .rasterization_state(&rasterisation)
        .multisample_state(&multisample)
        .depth_stencil_state(&depth_stencil)
        .color_blend_state(&colour_blend)
        .dynamic_state(&dynamic)
        .layout(layout)
        .push_next(&mut rendering);

    // SAFETY: every pointer in `info` points to locals alive until this call returns.
    let pipelines =
        unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None) };
    match pipelines {
        Ok(created) => Ok(created[0]),
        Err((_, e)) => Err(e.into()),
    }
}
