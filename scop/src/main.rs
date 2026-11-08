use std::env;
use std::process::ExitCode;

use engine::gpu::GpuContext;
use engine::mesh::{MeshData, Vertex};
use engine::pipeline::Shaders;
use engine::platform::{self, WindowConfig};

/// The mesh shaders, compiled by `build.rs` and embedded in the binary.
const MESH_SHADERS: Shaders = Shaders {
    vertex: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.vert.spv")),
    fragment: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.frag.spv")),
};

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
    match platform::run(config, MESH_SHADERS, quad(), print_gpu_info, pulsing_blue) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("scop: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Called once the window and the GPU are both set up (§ 3 and § 4 checkpoints).
fn print_gpu_info(gpu: &GpuContext) {
    let (major, minor, patch) = gpu.api_version();
    let (width, height) = gpu.extent();
    println!(
        "GPU: {} (Vulkan {major}.{minor}.{patch}), queue family {}",
        gpu.gpu_name(),
        gpu.queue_family_index()
    );
    println!(
        "swapchain: {:?}, {:?}, {width}x{height}, {} images",
        gpu.swapchain_format(),
        gpu.present_mode(),
        gpu.image_count()
    );
}

/// The § 5 checkpoint colour: a blue that pulses slowly (one cycle every ~6 s),
/// so you can see a new frame is drawn every time. Linear RGBA, 0.0–1.0.
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
