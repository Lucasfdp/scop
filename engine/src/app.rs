//! The contract between the engine and an app (§ 8): the `App` trait the engine
//! calls, and the small types an app uses to talk back — `Gfx` at setup, `Frame`
//! every frame, and handles instead of GPU objects.

use crate::gpu::GpuContext;
use crate::input::Input;
use crate::maths::Mat4;
use crate::mesh::MeshData;
use crate::pipeline::{DrawStyle, Shaders};
use crate::platform::RunError;
use crate::tga::Image;

/// What an app implements. The engine owns the loop and calls these
/// (inversion of control): `setup` once, then `update` and `draw` every frame.
pub trait App {
    /// Once, after the window and GPU exist: create pipelines and meshes.
    fn setup(&mut self, gfx: &mut Gfx<'_>) -> Result<(), RunError>;

    /// Every frame, before drawing. `dt` is seconds since the previous frame.
    /// `gfx` lets the app make (and destroy) meshes and textures mid-run (§ 19).
    /// Return `Flow::Exit` to quit. Default: carry on.
    fn update(&mut self, _input: &Input, _dt: f32, _gfx: &mut Gfx<'_>) -> Flow {
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
    /// Builds a pipeline from the app's compiled shaders, drawing solid triangles.
    pub fn create_pipeline(&mut self, shaders: Shaders) -> Result<PipelineId, RunError> {
        self.create_styled_pipeline(shaders, DrawStyle::Fill)
    }

    /// Builds a pipeline drawing in `style` (§ 18). Fails for a style this GPU
    /// can't do: check `supports` first.
    pub fn create_styled_pipeline(
        &mut self,
        shaders: Shaders,
        style: DrawStyle,
    ) -> Result<PipelineId, RunError> {
        Ok(self.gpu.create_pipeline(shaders, style)?)
    }

    /// Whether this GPU can draw in `style` (§ 18: wireframe is optional).
    pub fn supports(&self, style: DrawStyle) -> bool {
        self.gpu.supports(style)
    }

    /// Uploads a mesh to GPU memory (through a staging buffer, § 7).
    pub fn create_mesh(&mut self, data: &MeshData) -> Result<MeshId, RunError> {
        Ok(self.gpu.create_mesh(data)?)
    }

    /// Uploads a picture (e.g. from `load_tga`) as a texture.
    pub fn create_texture(&mut self, image: &Image) -> Result<TextureId, RunError> {
        Ok(TextureId(self.gpu.create_texture(image)?))
    }

    /// Frees a mesh made with `create_mesh` (§ 19: swapping models mid-run). Waits for
    /// the GPU to finish everything first, because frames still in flight may be
    /// drawing it (Q-19.3) — a short pause, so don't call it every frame. The id is
    /// never reused: drawing with it afterwards is an error, not someone else's mesh.
    pub fn destroy_mesh(&mut self, mesh: MeshId) -> Result<(), RunError> {
        Ok(self.gpu.destroy_mesh(mesh)?)
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
    /// Smallest and largest dot size for `DrawStyle::Points`, in pixels (§ 18).
    /// `(1.0, 1.0)` means the GPU only draws 1-pixel dots.
    pub point_size_range: (f32, f32),
}

/// One requested draw, as the app described it (tickets + plain numbers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DrawCall {
    pub(crate) pipeline: PipelineId,
    pub(crate) mesh: MeshId,
    pub(crate) texture: TextureId,
    pub(crate) model: Mat4,
    /// Four numbers for the shaders, meaning set by the app (§ 19). With
    /// `draw_textured`: `[blend, 0, 0, 0]` (§ 15).
    pub(crate) params: [f32; 4],
}

/// The draw list for one frame: what the app asks the engine to draw.
pub struct Frame {
    pub(crate) clear_colour: [f32; 4],
    pub(crate) aspect: f32,
    pub(crate) camera: Mat4,
    pub(crate) draws: Vec<DrawCall>,
    /// The window title the app asked for this frame, if any (§ 19).
    pub(crate) title: Option<String>,
}

impl Frame {
    /// `aspect` = width ÷ height of the image this frame will be drawn into.
    pub(crate) fn new(aspect: f32) -> Self {
        Frame {
            clear_colour: [0.0, 0.0, 0.0, 1.0],
            aspect,
            camera: Mat4::IDENTITY,
            draws: Vec::new(),
            title: None,
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

    /// The window's title (§ 19). Cheap to call every frame: the engine only passes
    /// it to the window when the text actually changes (title throttling).
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = Some(title.into());
    }

    /// Background colour (linear RGBA, 0.0–1.0). Default: black.
    pub fn clear(&mut self, colour: [f32; 4]) {
        self.clear_colour = colour;
    }

    /// Draw `mesh` with `pipeline`, placed in the world by `model`, in its vertex
    /// colours (built-in white texture, blend 0). Draws happen in request order.
    pub fn draw(&mut self, pipeline: PipelineId, mesh: MeshId, model: Mat4) {
        self.draw_textured(pipeline, mesh, TextureId(0), model, 0.0);
    }

    /// Like `draw`, with `texture` mixed over the vertex colours by `blend`:
    /// 0 = colours only, 1 = texture only (§ 14, § 15). `blend` is clamped to 0..1.
    pub fn draw_textured(
        &mut self,
        pipeline: PipelineId,
        mesh: MeshId,
        texture: TextureId,
        model: Mat4,
        blend: f32,
    ) {
        let params = [blend.clamp(0.0, 1.0), 0.0, 0.0, 0.0];
        self.draw_with(pipeline, mesh, texture, model, params);
    }

    /// The general form (§ 19): like `draw_textured`, but the app fills in all four
    /// push params itself — `params` arrives in the shaders as `push.params` (a
    /// `vec4`). The engine doesn't look inside: what each number means is the app's.
    pub fn draw_with(
        &mut self,
        pipeline: PipelineId,
        mesh: MeshId,
        texture: TextureId,
        model: Mat4,
        params: [f32; 4],
    ) {
        self.draws.push(DrawCall {
            pipeline,
            mesh,
            texture,
            model,
            params,
        });
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
        assert_eq!(frame.draws[1].mesh, MeshId(1));
        assert_eq!(frame.draws[1].texture, TextureId(0)); // plain draw = white texture
        assert_eq!(frame.draws[1].model, spin);
        assert_eq!(frame.draws[1].params, [0.0; 4]); // plain draw = colours only
        frame.draw_textured(PipelineId(0), MeshId(0), TextureId(1), spin, 7.0);
        assert_eq!(frame.draws[2].params, [1.0, 0.0, 0.0, 0.0]); // blend clamped
        frame.draw_with(
            PipelineId(0),
            MeshId(0),
            TextureId(1),
            spin,
            [7.0, 1.0, 2.0, 3.0],
        );
        assert_eq!(frame.draws[3].params, [7.0, 1.0, 2.0, 3.0]); // passed on untouched
        assert_eq!(frame.title, None);
        frame.set_title("scop");
        assert_eq!(frame.title.as_deref(), Some("scop"));
    }
}
