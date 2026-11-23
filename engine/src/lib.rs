//! engine — reusable rendering engine for scop and future projects.
//!
//! An app implements [`App`] and calls [`run`]. Everything an app may use is
//! re-exported below; every module is private, so the rest stays backstage (§ 8).

mod app;
mod buffer;
mod camera;
mod defaults;
mod depth;
mod fps;
mod frames;
mod gpu;
mod input;
mod maths;
mod mesh;
mod mesh_builder;
mod obj;
mod overlay;
mod pipeline;
mod platform;
mod probe;
mod shader_inputs;
mod shader_watch;
mod sky;
mod spirv;
mod swapchain;
mod texture;
mod tga;
mod transform;
mod triangulate;
mod tween;

// The whole public API.
pub use app::{App, Flow, Frame, Gfx, GpuInfo, InstanceRange, MeshId, PipelineId, TextureId};
pub use camera::{CameraSettings, Framing, OrbitCamera, Up, depth_range};
pub use defaults::{ColourMode, DEFAULT_SHADERS, DefaultParams, SKY_SHADERS, default_shader_files};
pub use fps::{FpsCounter, FpsReading};
pub use input::{Input, Key, MouseButton};
pub use maths::{Mat4, Vec3, Vec4};
pub use mesh::{MeshData, Vertex};
pub use mesh_builder::{
    LoadError, LoadedObj, UvMapping, box_uv, build_meshes, face_normals, load_obj,
};
pub use obj::{ObjError, ObjErrorKind, ObjModel, parse_obj};
pub use overlay::{GLYPH_HEIGHT, GLYPH_WIDTH, text_size};
pub use pipeline::{DrawStyle, Shaders};
pub use platform::{RunError, WindowConfig, run};
pub use shader_watch::{ShaderWatch, compile_shader};
pub use sky::{FACE_NAMES, Sky, generate_sky, load_cube_faces, sky_box};
pub use tga::{Image, TgaError, load_tga};
pub use transform::Transform;
pub use tween::{Easing, Tween, smoothstep};

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
