//! The contract between the engine and an app (§ 8): the `App` trait the engine
//! calls, and the small types an app uses to talk back — `Gfx` at setup, `Frame`
//! every frame, and handles instead of GPU objects.

use crate::defaults::{DEFAULT_SHADERS, SKY_SHADERS};
use crate::gpu::GpuContext;
use crate::input::Input;
use crate::maths::{Mat4, Vec3};
use crate::mesh::MeshData;
use crate::overlay;
use crate::pipeline::{DrawStyle, Shaders};
use crate::platform::RunError;
use crate::shader_inputs::INSTANCE_CAPACITY;
use crate::sky::{Sky, sky_box};
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
    pub fn create_pipeline(&mut self, shaders: Shaders<'_>) -> Result<PipelineId, RunError> {
        self.create_styled_pipeline(shaders, DrawStyle::Fill)
    }

    /// Builds a pipeline drawing in `style` (§ 18). Fails for a style this GPU
    /// can't do: check `supports` first.
    pub fn create_styled_pipeline(
        &mut self,
        shaders: Shaders<'_>,
        style: DrawStyle,
    ) -> Result<PipelineId, RunError> {
        Ok(self.gpu.create_pipeline(shaders, style)?)
    }

    /// Swaps `pipeline` for one built from new shaders (§ 21: hot reload). The new
    /// one is built first: on error, the old pipeline stays and keeps drawing. On
    /// success the GPU is waited for (a short pause) and the old one destroyed; the
    /// id stays the same, so draws that name it get the new version.
    pub fn replace_pipeline(
        &mut self,
        pipeline: PipelineId,
        shaders: Shaders<'_>,
        style: DrawStyle,
    ) -> Result<(), RunError> {
        Ok(self.gpu.replace_pipeline(pipeline, shaders, style)?)
    }

    /// A background pipeline (§ 23): solid, depth test on but depth **writing off**.
    /// Draw with it first (the sky): it paints the background and never blocks what
    /// comes after.
    pub fn create_background_pipeline(
        &mut self,
        shaders: Shaders<'_>,
    ) -> Result<PipelineId, RunError> {
        Ok(self.gpu.create_background_pipeline(shaders)?)
    }

    /// Uploads six square faces as one cube map (§ 23), in Vulkan's order: +x, −x,
    /// +y, −y, +z, −z. All faces must be square and the same size.
    pub fn create_cube_texture(&mut self, faces: [&Image; 6]) -> Result<TextureId, RunError> {
        Ok(TextureId(self.gpu.create_cube_texture(faces)?))
    }

    /// Makes `cube` the environment (§ 23): the cube map every shader can read as
    /// `environment` (set 0, binding 2). Fails if it isn't a cube map. Waits for the
    /// GPU (a short pause). Default: a white cube.
    pub fn set_environment(&mut self, cube: TextureId) -> Result<(), RunError> {
        Ok(self.gpu.set_environment(cube.0)?)
    }

    /// A pipeline with the engine's **default shaders** (§ 24) in `style`: fade,
    /// lighting, chrome and debug colours. Fill its four numbers with `DefaultParams`.
    pub fn create_default_pipeline(&mut self, style: DrawStyle) -> Result<PipelineId, RunError> {
        self.create_styled_pipeline(DEFAULT_SHADERS, style)
    }

    /// A ready-made sky (§ 24): the background pipeline with the default sky shaders,
    /// the sky box, and `faces` uploaded as a cube map — which also becomes the
    /// environment (what chrome reflects). Draw it with `Sky::draw`, first each frame.
    pub fn create_default_sky(&mut self, faces: [&Image; 6]) -> Result<Sky, RunError> {
        let pipeline = self.create_background_pipeline(SKY_SHADERS)?;
        let mesh = self.create_mesh(&sky_box())?;
        let cube = self.create_cube_texture(faces)?;
        self.set_environment(cube)?;
        Ok(Sky {
            pipeline,
            mesh,
            cube,
        })
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
    /// § 22: (first instance, how many). Ordinary draws: (0, 1) — position 0, the
    /// identity. Instanced draws: their place in the frame's instance list.
    pub(crate) instances: (u32, u32),
    /// § 25: drawn into the reflection probe's views too (false = reflective / text).
    pub(crate) in_reflections: bool,
}

/// Where one `draw_instances` put its copies in this frame's instance list (§ 22):
/// the first seat and how many. Only meaningful within the frame it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceRange {
    first: u32,
    count: u32,
}

/// The draw list for one frame: what the app asks the engine to draw.
pub struct Frame {
    pub(crate) clear_colour: [f32; 4],
    pub(crate) aspect: f32,
    pub(crate) camera: Mat4,
    pub(crate) draws: Vec<DrawCall>,
    /// The window title the app asked for this frame, if any (§ 19).
    pub(crate) title: Option<String>,
    /// The light (§ 20): direction it shines in (xyz) and strength (w). Default: off.
    pub(crate) light: [f32; 4],
    /// § 23: where the camera is (w = 1), for reflections. Default: the origin.
    pub(crate) eye: [f32; 4],
    /// § 22: every instanced draw's matrices, one after another. They go into the
    /// slot's instance list from position 1 (0 is the identity for ordinary draws).
    pub(crate) instances: Vec<Mat4>,
    /// Some copies didn't fit in the list and weren't drawn.
    pub(crate) instances_dropped: bool,
    /// § 25: capture a reflection probe this frame: (position, near, far).
    pub(crate) probe: Option<(Vec3, f32, f32)>,
    /// § 25: draws added while this is on are left out of the probe's views.
    pub(crate) reflective: bool,
    /// The image's size in pixels (for text, § 25 extra).
    pub(crate) size: (f32, f32),
    /// The engine's text pipeline, quad and font (`None` in tests: text is skipped).
    pub(crate) overlay: Option<Overlay>,
}

/// The engine's own resources for text on screen (§ 25 extra).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Overlay {
    pub(crate) pipeline: PipelineId,
    pub(crate) quad: MeshId,
    pub(crate) font: TextureId,
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
            light: [0.0; 4],
            eye: [0.0, 0.0, 0.0, 1.0],
            instances: Vec::new(),
            instances_dropped: false,
            probe: None,
            reflective: false,
            size: (1.0, 1.0),
            overlay: None,
        }
    }

    /// The image's size in pixels this frame (physical pixels: twice the window's
    /// points on a Retina screen). For placing text.
    pub fn size(&self) -> (f32, f32) {
        self.size
    }

    /// § 25: capture the scene into a reflection probe at `position` this frame —
    /// six extra views (costly: only ask when reflections are on screen). While a
    /// probe is captured, the environment every shader reads (chrome) is the probe's
    /// cube instead of the fixed one. `near` / `far`: the views' depth range.
    pub fn set_reflection_probe(&mut self, position: Vec3, near: f32, far: f32) {
        self.probe = Some((position, near, far));
    }

    /// § 25: draws added while `on` are left out of the probe's views — use it for the
    /// reflective model itself (the probe sits inside it, Q-25.1).
    pub fn set_reflective(&mut self, on: bool) {
        self.reflective = on;
    }

    /// § 25 extra: `text` on screen from (x, y) — pixels from the top-left — at
    /// `scale` pixels per font texel (1 = 8 × 13 per character). Drawn on top of
    /// everything that came before; never in reflections. `\n` starts a new line.
    pub fn draw_text(&mut self, text: &str, position: (f32, f32), scale: f32, colour: [f32; 3]) {
        let glyphs = overlay::layout_text(text, position, scale);
        self.draw_glyphs(&glyphs, colour);
    }

    /// § 25 extra: a solid rectangle on screen (pixels), e.g. behind text.
    pub fn draw_panel(
        &mut self,
        (x, y): (f32, f32),
        (width, height): (f32, f32),
        colour: [f32; 3],
    ) {
        self.draw_glyphs(&[overlay::panel(x, y, width, height)], colour);
    }

    fn draw_glyphs(&mut self, glyphs: &[overlay::Glyph], colour: [f32; 3]) {
        let Some(kit) = self.overlay else {
            return;
        };
        let copies: Vec<Mat4> = glyphs
            .iter()
            .map(|g| overlay::glyph_instance(g, colour))
            .collect();
        let params = [self.size.0, self.size.1, 0.0, 0.0]; // the text shader needs the size
        let was = std::mem::replace(&mut self.reflective, true); // never in reflections
        self.draw_instances(
            kit.pipeline,
            kit.quad,
            kit.font,
            Mat4::IDENTITY,
            params,
            &copies,
        );
        self.reflective = was;
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

    /// One directional light for this frame (§ 20), like the sun: `direction` is the
    /// way it shines (from the light towards the scene, any length), `strength` 0..1,
    /// 0 = off. It reaches the shaders with the camera, as `camera.light`; what the
    /// shaders do with it is the app's choice. Default: off.
    pub fn set_light(&mut self, direction: Vec3, strength: f32) {
        let d = direction.normalise();
        self.light = [d.x, d.y, d.z, strength.clamp(0.0, 1.0)];
    }

    /// Where the camera stands this frame (§ 23). Reaches the shaders as `camera.eye`:
    /// what reflections are worked out from. Default: the origin.
    pub fn set_camera_position(&mut self, eye: Vec3) {
        self.eye = [eye.x, eye.y, eye.z, 1.0];
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
            instances: (0, 1),
            in_reflections: !self.reflective,
        });
    }

    /// Instancing (§ 22): draws `mesh` once per matrix in `copies`, with **one** draw
    /// call. Each copy is placed by `model × its own matrix` (the shader reads it with
    /// `gl_InstanceIndex`); `params` and the picture are shared. If this frame's list
    /// is full (`INSTANCE_CAPACITY`), only the copies that fit are drawn, and the
    /// engine says so once.
    ///
    /// Returns where the copies went in the list (`None` if none fitted): pass it to
    /// `draw_instances_again` to draw another mesh with the **same** copies without
    /// storing their matrices twice.
    pub fn draw_instances(
        &mut self,
        pipeline: PipelineId,
        mesh: MeshId,
        texture: TextureId,
        model: Mat4,
        params: [f32; 4],
        copies: &[Mat4],
    ) -> Option<InstanceRange> {
        // Position 0 is the identity, so the list has `capacity − 1` places for copies.
        let free = INSTANCE_CAPACITY - 1 - self.instances.len();
        let count = copies.len().min(free);
        if count < copies.len() {
            self.instances_dropped = true;
        }
        if count == 0 {
            return None;
        }
        let first = 1 + self.instances.len() as u32; // after the identity and earlier copies
        self.instances.extend_from_slice(&copies[..count]);
        let range = InstanceRange {
            first,
            count: count as u32,
        };
        self.draw_instances_again(pipeline, mesh, texture, model, params, range);
        Some(range)
    }

    /// Draws `mesh` with copies already in this frame's list (§ 22, shared range): the
    /// `range` an earlier `draw_instances` returned **this frame**. No matrices are
    /// added, so the 2nd, 3rd… object of a crowd costs no seats. A range that isn't in
    /// this frame's list (e.g. kept from an earlier frame) draws nothing.
    pub fn draw_instances_again(
        &mut self,
        pipeline: PipelineId,
        mesh: MeshId,
        texture: TextureId,
        model: Mat4,
        params: [f32; 4],
        range: InstanceRange,
    ) {
        // Seats 1..=len hold this frame's copies; anything else isn't ours to read.
        let end = range.first as usize + range.count as usize;
        if range.first == 0 || end > 1 + self.instances.len() {
            return;
        }
        self.draws.push(DrawCall {
            pipeline,
            mesh,
            texture,
            model,
            params,
            instances: (range.first, range.count),
            in_reflections: !self.reflective,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflective_draws_stay_out_of_the_probe() {
        let mut frame = Frame::new(1.0);
        assert_eq!(frame.probe, None); // no probe → the fixed environment
        frame.set_reflection_probe(Vec3::new(1.0, 2.0, 3.0), 0.1, 50.0);
        frame.draw(PipelineId(0), MeshId(0), Mat4::IDENTITY);
        frame.set_reflective(true);
        frame.draw(PipelineId(0), MeshId(1), Mat4::IDENTITY);
        frame.set_reflective(false);
        frame.draw(PipelineId(0), MeshId(2), Mat4::IDENTITY);
        let kept: Vec<_> = frame.draws.iter().map(|d| d.in_reflections).collect();
        assert_eq!(kept, vec![true, false, true]);
        assert_eq!(frame.probe, Some((Vec3::new(1.0, 2.0, 3.0), 0.1, 50.0)));
    }

    #[test]
    fn text_is_never_reflected_and_leaves_the_flag_alone() {
        let mut frame = Frame::new(1.0);
        frame.draw_text("hi", (0.0, 0.0), 1.0, [1.0; 3]); // no kit (tests): skipped
        assert!(frame.draws.is_empty());
        frame.overlay = Some(Overlay {
            pipeline: PipelineId(7),
            quad: MeshId(8),
            font: TextureId(9),
        });
        frame.draw_text("hi", (0.0, 0.0), 1.0, [1.0; 3]);
        let text = &frame.draws[0];
        assert_eq!((text.mesh, text.instances.1), (MeshId(8), 2)); // one copy per glyph
        assert!(!text.in_reflections);
        assert!(!frame.reflective);
    }

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
        assert_eq!(frame.draws[3].instances, (0, 1)); // ordinary draw: the identity
        assert_eq!(frame.title, None);
        frame.set_title("scop");
        assert_eq!(frame.title.as_deref(), Some("scop"));
    }
    #[test]
    fn instanced_draws_pack_after_the_identity() {
        let mut frame = Frame::new(1.0);
        let copies = |n: usize| vec![Mat4::IDENTITY; n];
        frame.draw(PipelineId(0), MeshId(0), Mat4::IDENTITY); // ordinary: position 0
        frame.draw_instances(
            PipelineId(0),
            MeshId(0),
            TextureId(0),
            Mat4::IDENTITY,
            [0.0; 4],
            &copies(3),
        );
        frame.draw_instances(
            PipelineId(0),
            MeshId(1),
            TextureId(0),
            Mat4::IDENTITY,
            [0.0; 4],
            &copies(2),
        );
        let ranges: Vec<_> = frame.draws.iter().map(|d| d.instances).collect();
        // Ordinary at 0; the first crowd at 1..4; the second straight after, at 4..6.
        assert_eq!(ranges, vec![(0, 1), (1, 3), (4, 2)]);
        assert_eq!(frame.instances.len(), 5);
        assert!(!frame.instances_dropped);
    }

    #[test]
    fn later_objects_share_the_first_objects_copies() {
        let mut frame = Frame::new(1.0);
        let crowd = vec![Mat4::IDENTITY; 512];
        let range = frame
            .draw_instances(
                PipelineId(0),
                MeshId(0),
                TextureId(0),
                Mat4::IDENTITY,
                [0.0; 4],
                &crowd,
            )
            .unwrap();
        for mesh in 1..10 {
            frame.draw_instances_again(
                PipelineId(0),
                MeshId(mesh),
                TextureId(0),
                Mat4::IDENTITY,
                [0.0; 4],
                range,
            );
        }
        // 10 objects, one set of 512 seats — not 5,120.
        assert_eq!(frame.instances.len(), 512);
        assert!(frame.draws.iter().all(|d| d.instances == (1, 512)));
        assert!(!frame.instances_dropped);
        // A range that isn't in this frame's list draws nothing.
        let mut next_frame = Frame::new(1.0);
        next_frame.draw_instances_again(
            PipelineId(0),
            MeshId(0),
            TextureId(0),
            Mat4::IDENTITY,
            [0.0; 4],
            range,
        );
        assert!(next_frame.draws.is_empty());
    }

    #[test]
    fn too_many_copies_draw_what_fits() {
        let mut frame = Frame::new(1.0);
        let all = vec![Mat4::IDENTITY; INSTANCE_CAPACITY + 10];
        frame.draw_instances(
            PipelineId(0),
            MeshId(0),
            TextureId(0),
            Mat4::IDENTITY,
            [0.0; 4],
            &all,
        );
        // Everything but position 0 (the identity) is used, and no further.
        assert_eq!(
            frame.draws[0].instances,
            (1, (INSTANCE_CAPACITY - 1) as u32)
        );
        assert!(frame.instances_dropped);
        // The list is full: a later instanced draw is skipped, not overrun.
        frame.draw_instances(
            PipelineId(0),
            MeshId(0),
            TextureId(0),
            Mat4::IDENTITY,
            [0.0; 4],
            &all[..1],
        );
        assert_eq!(frame.draws.len(), 1);
    }
}
