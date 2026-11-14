//! scop — a `.obj` viewer built on the `engine` crate. scop only says *what* to
//! show and how keys behave; the engine does everything GPU- and window-related.

use std::env;
use std::process::ExitCode;

use engine::{
    App, Flow, Frame, Gfx, GpuInfo, Input, Key, Mat4, MeshData, MeshId, ObjModel, PipelineId,
    RunError, Shaders, Vec3, Vertex, WindowConfig,
};

/// The mesh shaders, compiled by `build.rs` and embedded in the binary.
const MESH_SHADERS: Shaders = Shaders {
    vertex: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.vert.spv")),
    fragment: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.frag.spv")),
};

/// How fast the model turns, in radians per second (about one turn every 8 s).
const SPIN_SPEED: f32 = 0.8;
/// Vertical field of view of the camera.
const FOV_Y_DEGREES: f32 = 60.0;
/// Background colour (linear RGBA): dark grey, so every face colour shows.
const BACKGROUND: [f32; 4] = [0.02, 0.02, 0.03, 1.0];

/// scop's state. The ids are `None` until the engine calls `setup`.
struct Scop {
    /// The model's vertices and triangles, ready to upload in `setup`.
    mesh: MeshData,
    /// Middle of the model's bounding box: what it spins around (§ 12, Q-9.3).
    centre: Vec3,
    /// Size of the model: decides how far back the camera sits.
    radius: f32,
    /// How far the model has turned so far, in radians.
    angle: f32,
    pipeline: Option<PipelineId>,
    model: Option<MeshId>,
}

impl App for Scop {
    fn setup(&mut self, gfx: &mut Gfx<'_>) -> Result<(), RunError> {
        print_gpu_info(&gfx.info());
        self.pipeline = Some(gfx.create_pipeline(MESH_SHADERS)?);
        self.model = Some(gfx.create_mesh(&self.mesh)?);
        Ok(())
    }

    fn update(&mut self, input: &Input, dt: f32) -> Flow {
        // × dt: the same speed on a 30 fps machine and a 120 fps one.
        self.angle += SPIN_SPEED * dt;
        // Quitting is scop's decision now, not the engine's.
        if input.pressed(Key::Escape) {
            Flow::Exit
        } else {
            Flow::Continue
        }
    }

    fn draw(&self, frame: &mut Frame) {
        frame.clear(BACKGROUND);

        // Framing: far enough back that a sphere of `radius` fits the view (§ 12).
        let fov_y = FOV_Y_DEGREES.to_radians();
        let distance = self.radius / (fov_y / 2.0).sin();
        let near = self.radius * 0.01; // as large as possible, for depth precision (§ 11)
        let far = distance + self.radius * 2.0; // the back of the model is never clipped

        // Camera, once per frame (the noticeboard). Aspect is read fresh every frame.
        let projection = Mat4::perspective(fov_y, frame.aspect(), near, far);
        let view = Mat4::translation(Vec3::new(0.0, 0.0, -distance));
        frame.set_camera(projection * view);

        // The model's placement (the sticky note). Right to left: move its centre
        // onto the origin, then turn it there (Q-9.3 / Q-12.5).
        let model = Mat4::rotation_y(self.angle) * Mat4::translation(-self.centre);
        if let (Some(pipeline), Some(mesh)) = (self.pipeline, self.model) {
            frame.draw(pipeline, mesh, model);
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    let Some(path) = args.get(1) else {
        eprintln!("usage: scop <file.obj>");
        return ExitCode::FAILURE;
    };

    let text = match engine::read_file(path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("scop: {path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    let obj = match engine::parse_obj(&text) {
        Ok(obj) => obj,
        Err(err) => {
            eprintln!("scop: {path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "{path}: {} corners, {} triangles",
        obj.positions.len(),
        obj.triangles.len()
    );

    let config = WindowConfig {
        title: String::from("scop"),
        width: 1280,
        height: 720,
    };
    let scop = Scop {
        mesh: mesh_from_obj(&obj),
        centre: obj.centre(),
        // A model that's a single point would put the camera on top of it.
        radius: obj.radius().max(0.001),
        angle: 0.0,
        pipeline: None,
        model: None,
    };
    match engine::run(config, scop) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("scop: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Printed once the window and the GPU are set up (§ 3, § 4 and § 11 checkpoints).
fn print_gpu_info(info: &GpuInfo) {
    let (major, minor, patch) = info.vulkan_version;
    let (width, height) = info.extent;
    println!(
        "GPU: {} (Vulkan {major}.{minor}.{patch}), queue family {}",
        info.name, info.queue_family
    );
    println!(
        "swapchain: {}, {}, {width}x{height}, {} images",
        info.swapchain_format, info.present_mode, info.image_count
    );
    println!("depth buffer: {}", info.depth_format);
}

/// The parsed model as GPU-ready mesh data. Colour per corner from its place in the
/// bounding box (x → red, y → green, z → blue), like § 10's cube, so the shape is
/// readable. § 13 replaces this with a grey per face.
fn mesh_from_obj(obj: &ObjModel) -> MeshData {
    let size = obj.max - obj.min;
    // 0..1 inside the box; a flat side (size 0) gets the middle value.
    let fraction = |value: f32, min: f32, size: f32| {
        if size > 0.0 {
            (value - min) / size
        } else {
            0.5
        }
    };
    let vertices = obj
        .positions
        .iter()
        .map(|p| Vertex {
            position: [p.x, p.y, p.z],
            colour: [
                fraction(p.x, obj.min.x, size.x),
                fraction(p.y, obj.min.y, size.y),
                fraction(p.z, obj.min.z, size.z),
            ],
        })
        .collect();
    let indices = obj.triangles.iter().flatten().copied().collect();
    MeshData { vertices, indices }
}
