//! scop — a `.obj` viewer built on the `engine` crate. scop only says *what* to
//! show and how keys behave; the engine does everything GPU- and window-related.

use std::env;
use std::process::ExitCode;

use engine::{
    App, Flow, Frame, Gfx, GpuInfo, Input, Key, Mat4, MeshData, MeshId, PipelineId, RunError,
    Shaders, Vec3, Vertex, WindowConfig,
};

/// The mesh shaders, compiled by `build.rs` and embedded in the binary.
const MESH_SHADERS: Shaders = Shaders {
    vertex: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.vert.spv")),
    fragment: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.frag.spv")),
};

/// How fast the cube turns, in radians per second (about one turn every 8 s).
const SPIN_SPEED: f32 = 0.8;
/// Vertical field of view of the camera.
const FOV_Y_DEGREES: f32 = 60.0;
/// How far the camera sits back from the cube.
const CAMERA_DISTANCE: f32 = 3.0;
/// Background colour (linear RGBA): dark grey, so every face colour shows.
const BACKGROUND: [f32; 4] = [0.02, 0.02, 0.03, 1.0];

/// scop's state. The ids are `None` until the engine calls `setup`.
struct Scop {
    /// How far the cube has turned so far, in radians.
    angle: f32,
    pipeline: Option<PipelineId>,
    cube: Option<MeshId>,
}

impl App for Scop {
    fn setup(&mut self, gfx: &mut Gfx<'_>) -> Result<(), RunError> {
        print_gpu_info(&gfx.info());
        self.pipeline = Some(gfx.create_pipeline(MESH_SHADERS)?);
        self.cube = Some(gfx.create_mesh(&cube())?);
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

        // Camera, once per frame (the noticeboard). Aspect is read fresh every frame.
        let projection = Mat4::perspective(FOV_Y_DEGREES.to_radians(), frame.aspect(), 0.1, 100.0);
        let view = Mat4::translation(Vec3::new(0.0, 0.0, -CAMERA_DISTANCE));
        frame.set_camera(projection * view);

        // This object's placement (the sticky note). The cube is already centred on
        // the origin, so it just turns: mostly around y, a little around x so the top
        // and bottom faces show too. Right to left: x-turn first, then y-turn.
        let model = Mat4::rotation_y(self.angle) * Mat4::rotation_x(self.angle * 0.5);
        if let (Some(pipeline), Some(cube)) = (self.pipeline, self.cube) {
            frame.draw(pipeline, cube, model);
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
    println!("{path}: {} lines", text.lines().count());

    let config = WindowConfig {
        title: String::from("scop"),
        width: 1280,
        height: 720,
    };
    let scop = Scop {
        angle: 0.0,
        pipeline: None,
        cube: None,
    };
    match engine::run(config, scop) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("scop: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Printed once the window and the GPU are set up (§ 3 and § 4 checkpoints).
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

/// The § 10 checkpoint shape: a cube from 8 shared corners and 36 indices
/// (6 faces × 2 triangles × 3). Centred on the origin, 1 unit wide. Each corner's
/// colour comes from its position (x → red, y → green, z → blue), so every corner
/// differs. Triangles are counter-clockwise seen from outside (§ 11 will care).
/// § 12 replaces this with the `.obj`.
fn cube() -> MeshData {
    let corner = |x: f32, y: f32, z: f32| Vertex {
        position: [x, y, z],
        colour: [x + 0.5, y + 0.5, z + 0.5],
    };
    MeshData {
        vertices: vec![
            corner(-0.5, -0.5, -0.5), // 0: left  bottom back
            corner(0.5, -0.5, -0.5),  // 1: right bottom back
            corner(0.5, 0.5, -0.5),   // 2: right top    back
            corner(-0.5, 0.5, -0.5),  // 3: left  top    back
            corner(-0.5, -0.5, 0.5),  // 4: left  bottom front
            corner(0.5, -0.5, 0.5),   // 5: right bottom front
            corner(0.5, 0.5, 0.5),    // 6: right top    front
            corner(-0.5, 0.5, 0.5),   // 7: left  top    front
        ],
        #[rustfmt::skip]
        indices: vec![
            4, 5, 6, 6, 7, 4, // front  (+z)
            1, 0, 3, 3, 2, 1, // back   (−z)
            5, 1, 2, 2, 6, 5, // right  (+x)
            0, 4, 7, 7, 3, 0, // left   (−x)
            7, 6, 2, 2, 3, 7, // top    (+y)
            0, 1, 5, 5, 4, 0, // bottom (−y)
        ],
    }
}
