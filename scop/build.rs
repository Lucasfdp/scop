//! Build script: compiles every shader in `shaders/` to SPIR-V with `glslc`,
//! before scop itself is compiled. Output goes to Cargo's `OUT_DIR`, where
//! `main.rs` embeds it with `include_bytes!`.
//!
//! § 16, option C: compiled copies are also committed in `shaders/spv/`.
//! - `glslc` found  → compile; warn if the committed copy is missing or stale
//!   (refresh with `make spv`).
//! - `glslc` missing → use the committed copy, with a warning, so the build never
//!   fails just because the shader compiler isn't installed (e.g. on campus).

use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Run this script again whenever anything in shaders/ (including spv/) changes.
    println!("cargo:rerun-if-changed=shaders");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo always sets OUT_DIR"));
    let entries = fs::read_dir("shaders").expect("scop/shaders/ should exist");

    for entry in entries {
        let path = entry.expect("readable directory entry").path();
        let is_shader = matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("vert" | "frag")
        );
        if !is_shader {
            continue;
        }
        let name = path
            .file_name()
            .expect("a file has a name")
            .to_string_lossy()
            .into_owned();
        let output = out_dir.join(format!("{name}.spv"));
        let committed = Path::new("shaders/spv").join(format!("{name}.spv"));

        match Command::new("glslc")
            .arg(&path)
            .arg("-o")
            .arg(&output)
            .status()
        {
            Ok(status) if status.success() => check_committed(&output, &committed, &name),
            // A real compile error: stop the build. glslc's own message (file:line:
            // error) is shown above this one. A failing build script panics.
            Ok(_) => panic!("glslc failed to compile {}", path.display()),
            Err(e) if e.kind() == ErrorKind::NotFound => use_committed(&committed, &output, &name),
            Err(e) => panic!("could not run glslc: {e}"),
        }
    }
}

/// `glslc` worked: the committed fallback should hold the same bytes.
fn check_committed(fresh: &Path, committed: &Path, name: &str) {
    match (fs::read(fresh), fs::read(committed)) {
        (Ok(a), Ok(b)) if a == b => {}
        (Ok(_), Ok(_)) => println!(
            "cargo:warning=shaders/spv/{name}.spv is stale (shader changed): run `make spv` and commit it"
        ),
        _ => println!(
            "cargo:warning=shaders/spv/{name}.spv is missing: run `make spv` and commit it"
        ),
    }
}

/// No `glslc`: copy the committed SPIR-V instead of compiling.
fn use_committed(committed: &Path, output: &Path, name: &str) {
    if let Err(e) = fs::copy(committed, output) {
        panic!(
            "glslc isn't installed and there's no committed shaders/spv/{name}.spv ({e}). \
             Install glslc (`make deps` tells you how), then run `make spv`."
        );
    }
    println!("cargo:warning=glslc not found: using the committed shaders/spv/{name}.spv");
}
