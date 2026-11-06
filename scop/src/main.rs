use std::env;
use std::process::ExitCode;

use engine::gpu::GpuContext;
use engine::platform::{self, WindowConfig};

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

    // Before the window for now; § 4 moves this into the window's `resumed`.
    let gpu = match GpuContext::new(c"scop") {
        Ok(gpu) => gpu,
        Err(err) => {
            eprintln!("scop: {err}");
            return ExitCode::FAILURE;
        }
    };
    let (major, minor, patch) = gpu.api_version();
    println!(
        "GPU: {} (Vulkan {major}.{minor}.{patch}), graphics queue family {}",
        gpu.gpu_name(),
        gpu.queue_family_index()
    );

    let config = WindowConfig {
        title: String::from("scop"),
        width: 1280,
        height: 720,
    };
    match platform::run(config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("scop: {err}");
            ExitCode::FAILURE
        }
    }
}
