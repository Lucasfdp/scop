//! scop — a `.obj` viewer built on the `engine` crate. scop only says *what* to
//! show and how keys behave; the engine does everything GPU- and window-related.

use std::env;
use std::process::ExitCode;
use std::time::Instant;

use engine::{
    App, Flow, Frame, Gfx, GpuInfo, Input, Key, MeshData, MeshId, PipelineId, RunError, Shaders,
    Vertex, WindowConfig,
};

/// The mesh shaders, compiled by `build.rs` and embedded in the binary.
const MESH_SHADERS: Shaders = Shaders {
    vertex: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.vert.spv")),
    fragment: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.frag.spv")),
};

/// scop's state. The ids are `None` until the engine calls `setup`.
struct Scop {
    start: Instant,
    pipeline: Option<PipelineId>,
    quad: Option<MeshId>,
}

impl App for Scop {
    fn setup(&mut self, gfx: &mut Gfx<'_>) -> Result<(), RunError> {
        print_gpu_info(&gfx.info());
        self.pipeline = Some(gfx.create_pipeline(MESH_SHADERS)?);
        self.quad = Some(gfx.create_mesh(&quad())?);
        Ok(())
    }

    fn update(&mut self, input: &Input, _dt: f32) -> Flow {
        // Quitting is scop's decision now, not the engine's.
        if input.pressed(Key::Escape) {
            Flow::Exit
        } else {
            Flow::Continue
        }
    }

    fn draw(&self, frame: &mut Frame) {
        frame.clear(pulsing_blue(self.start.elapsed().as_secs_f32()));
        if let (Some(pipeline), Some(quad)) = (self.pipeline, self.quad) {
            frame.draw(pipeline, quad);
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
        start: Instant::now(),
        pipeline: None,
        quad: None,
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
}

/// The § 5 checkpoint colour: a blue that pulses slowly (one cycle every ~6 s).
/// Linear RGBA, 0.0–1.0.
fn pulsing_blue(seconds: f32) -> [f32; 4] {
    let wave = 0.5 + 0.5 * seconds.sin(); // 0.0 → 1.0 → 0.0 …
    [0.05, 0.1 + 0.2 * wave, 0.3 + 0.6 * wave, 1.0]
}

/// The § 7 checkpoint shape: a square from 4 shared corners and 6 indices.
/// (Vulkan's y points down: −0.5 is the top.) § 12 replaces this with the `.obj`.
fn quad() -> MeshData {
    MeshData {
        vertices: vec![
            Vertex {
                position: [-0.5, -0.5],
                colour: [1.0, 0.0, 0.0],
            }, // 0: top left, red
            Vertex {
                position: [0.5, -0.5],
                colour: [0.0, 1.0, 0.0],
            }, // 1: top right, green
            Vertex {
                position: [0.5, 0.5],
                colour: [0.0, 0.0, 1.0],
            }, // 2: bottom right, blue
            Vertex {
                position: [-0.5, 0.5],
                colour: [1.0, 1.0, 0.0],
            }, // 3: bottom left, yellow
        ],
        // § 7 checkpoint step 6: change one of these and watch the shape change.
        indices: vec![0, 1, 2, 2, 3, 0],
    }
}
