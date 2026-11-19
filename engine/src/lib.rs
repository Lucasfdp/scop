//! engine — reusable rendering engine for scop and future projects.
//!
//! An app implements [`App`] and calls [`run`]. Everything an app may use is
//! re-exported below; every module is private, so the rest stays backstage (§ 8).

mod app;
mod buffer;
mod depth;
mod fps;
mod frames;
mod gpu;
mod input;
mod maths;
mod mesh;
mod obj;
mod pipeline;
mod platform;
mod shader_inputs;
mod spirv;
mod swapchain;
mod texture;
mod tga;
mod triangulate;

// The whole public API.
pub use app::{App, Flow, Frame, Gfx, GpuInfo, MeshId, PipelineId, TextureId};
pub use fps::{FpsCounter, FpsReading};
pub use input::{Input, Key, MouseButton};
pub use maths::{Mat4, Vec3, Vec4};
pub use mesh::{MeshData, Vertex};
pub use obj::{ObjError, ObjErrorKind, ObjModel, parse_obj};
pub use pipeline::{DrawStyle, Shaders};
pub use platform::{RunError, WindowConfig, run};
pub use tga::{Image, TgaError, load_tga};

use std::fs;
use std::io;

/// Reads the whole file at `path` into a `String`.
///
/// Returns an `io::Error` if the file is missing, unreadable or not UTF-8.
pub fn read_file(path: &str) -> io::Result<String> {
    fs::read_to_string(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_an_error_not_a_panic() {
        assert!(read_file("definitely/not/here.obj").is_err());
    }

    #[test]
    fn reads_existing_file() {
        let text = read_file("../resources/42.obj").expect("42.obj should be readable");
        assert!(text.lines().count() > 0);
    }
}
