//! The contract between the engine and an app (§ 8): the `App` trait the engine
//! calls, and the small types an app uses to talk back — `Gfx` at setup, `Frame`
//! every frame, and handles instead of GPU objects.

use crate::gpu::GpuContext;
use crate::input::Input;
use crate::mesh::MeshData;
use crate::pipeline::Shaders;
use crate::platform::RunError;

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
    /// Swapchain size in physical pixels.
    pub extent: (u32, u32),
    pub image_count: usize,
}

/// The draw list for one frame: what the app asks the engine to draw.
pub struct Frame {
    pub(crate) clear_colour: [f32; 4],
    pub(crate) draws: Vec<(PipelineId, MeshId)>,
}

impl Frame {
    pub(crate) fn new() -> Self {
        Frame {
            clear_colour: [0.0, 0.0, 0.0, 1.0],
            draws: Vec::new(),
        }
    }

    /// Background colour (linear RGBA, 0.0–1.0). Default: black.
    pub fn clear(&mut self, colour: [f32; 4]) {
        self.clear_colour = colour;
    }

    /// Draw `mesh` with `pipeline`. Draws happen in the order they're requested.
    pub fn draw(&mut self, pipeline: PipelineId, mesh: MeshId) {
        self.draws.push((pipeline, mesh));
    }
}
