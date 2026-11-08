//! Build script: compiles every shader in `shaders/` to SPIR-V with `glslc`,
//! before scop itself is compiled. Output goes to Cargo's `OUT_DIR`, where
//! `main.rs` embeds it with `include_bytes!`.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Run this script again whenever anything in shaders/ changes.
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
            .to_string_lossy();
        let output = out_dir.join(format!("{name}.spv"));

        // A failing build script stops the build; panicking is how it fails.
        // glslc's own error message (file:line: error) is shown above this one.
        match Command::new("glslc")
            .arg(&path)
            .arg("-o")
            .arg(&output)
            .status()
        {
            Ok(status) if status.success() => {}
            Ok(_) => panic!("glslc failed to compile {}", path.display()),
            Err(e) => panic!("could not run glslc ({e}) — install it: brew install shaderc"),
        }
    }
}
