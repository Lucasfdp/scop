//! The contract between the engine and an app (§ 8): the `App` trait the engine
//! calls, and the small types an app uses to talk back — `Gfx` at setup, `Frame`
//! every frame, and handles instead of GPU objects.

use crate::gpu::GpuContext;
use crate::input::Input;
use crate::maths::Mat4;
use crate::mesh::MeshData;
use crate::pipeline::Shaders;
use crate::platform::RunError;
use crate::tga::Image;

/// What an app implements. The engine owns the loop and calls these
/// (inversion of control): `setup` once, then `update` and `draw` every frame.
pub trait App {
    /// Once, after the window and GPU exist: create pipelines and meshes.
    fn setup(&mut self, gfx: &mut Gfx<'_>) -> Result<(), RunError>;

    /// Every frame, before drawing. `dt` is seconds since the previous frame.
    /// Return `Flow::Exit` to quit. Default: carry on.
    fn update(&mut self, _input: &Input, _dt: f32) -> Flow {
        Flow::Continue
    }

    /// Every frame: describe what to draw by filling in `frame`.
    fn draw(&self, frame: &mut Frame);
}

/// Whether the app wants to keep running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Exit,
}

/// Ticket for a pipeline the engine keeps. The number inside is engine-only,
/// so an app can't make one up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineId(pub(crate) usize);

/// Ticket for a mesh the engine keeps (in GPU memory).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshId(pub(crate) usize);

/// Ticket for a texture the engine keeps (§ 14). `TextureId(0)` is the built-in
/// 1×1 white texture, used by `Frame::draw`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureId(pub(crate) usize);

/// The app's access to the GPU during `setup`.
pub struct Gfx<'a> {
    pub(crate) gpu: &'a mut GpuContext,
}

impl Gfx<'_> {
    /// Builds a pipeline from the app's compiled shaders.
    pub fn create_pipeline(&mut self, shaders: Shaders) -> Result<PipelineId, RunError> {
        Ok(self.gpu.create_pipeline(shaders)?)
    }

    /// Uploads a mesh to GPU memory (through a staging buffer, § 7).
    pub fn create_mesh(&mut self, data: &MeshData) -> Result<MeshId, RunError> {
        Ok(self.gpu.create_mesh(data)?)
    }

    /// Uploads a picture (e.g. from `load_tga`) as a texture.
    pub fn create_texture(&mut self, image: &Image) -> Result<TextureId, RunError> {
        Ok(TextureId(self.gpu.create_texture(image)?))
    }

    /// Facts about the GPU and swapchain, for printing.
    pub fn info(&self) -> GpuInfo {
        self.gpu.info()
    }
}

/// Facts about the GPU and swapchain, as plain values (no Vulkan types).
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    /// (major, minor, patch)
    pub vulkan_version: (u32, u32, u32),
    pub queue_family: u32,
    pub swapchain_format: String,
    pub present_mode: String,
    /// The depth buffer's format (§ 11), e.g. `D32_SFLOAT`.
    pub depth_format: String,
    /// Swapchain size in physical pixels.
    pub extent: (u32, u32),
    pub image_count: usize,
}

/// The draw list for one frame: what the app asks the engine to draw.
pub struct Frame {
    pub(crate) clear_colour: [f32; 4],
    pub(crate) aspect: f32,
    pub(crate) camera: Mat4,
    pub(crate) draws: Vec<(PipelineId, MeshId, TextureId, Mat4)>,
}

impl Frame {
    /// `aspect` = width ÷ height of the image this frame will be drawn into.
    pub(crate) fn new(aspect: f32) -> Self {
        Frame {
            clear_colour: [0.0, 0.0, 0.0, 1.0],
            aspect,
            camera: Mat4::IDENTITY,
            draws: Vec::new(),
        }
    }

    /// Width ÷ height of this frame's image, for `Mat4::perspective`. Read it
    /// every frame, never store it: it changes when the window is resized (Q-10.5).
    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    /// The camera for every draw this frame: `projection * view`. Default: identity
    /// (corners are used as-is, in Vulkan's −1..+1 screen box).
    pub fn set_camera(&mut self, view_proj: Mat4) {
        self.camera = view_proj;
    }

    /// Background colour (linear RGBA, 0.0–1.0). Default: black.
    pub fn clear(&mut self, colour: [f32; 4]) {
        self.clear_colour = colour;
    }

    /// Draw `mesh` with `pipeline`, placed in the world by `model`, with the built-in
    /// white texture. Draws happen in the order they're requested.
    pub fn draw(&mut self, pipeline: PipelineId, mesh: MeshId, model: Mat4) {
        self.draw_textured(pipeline, mesh, TextureId(0), model);
    }

    /// Like `draw`, reading `texture` in the fragment shader (§ 14).
    pub fn draw_textured(
        &mut self,
        pipeline: PipelineId,
        mesh: MeshId,
        texture: TextureId,
        model: Mat4,
    ) {
        self.draws.push((pipeline, mesh, texture, model));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_keeps_camera_and_each_draws_model() {
        let mut frame = Frame::new(2.0);
        assert_eq!(frame.aspect(), 2.0);
        assert_eq!(frame.camera, Mat4::IDENTITY); // default when the app sets none
        let spin = Mat4::rotation_y(1.0);
        frame.set_camera(spin);
        frame.draw(PipelineId(0), MeshId(0), Mat4::IDENTITY);
        frame.draw(PipelineId(0), MeshId(1), spin);
        assert_eq!(frame.camera, spin);
        assert_eq!(
            frame.draws[1],
            (PipelineId(0), MeshId(1), TextureId(0), spin)
        );
    }
}
