use std::env;
use std::process::ExitCode;

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
