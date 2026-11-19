//! Shader hot reload (§ 21): notice a saved shader, compile it with `glslc`, hand
//! the SPIR-V back. Swapping the pipelines is `Gfx::replace_pipeline`'s job.
//!
//! Only `std`: polling modification times (no file-watching library — the subject
//! only allows window and event libraries) and running `glslc` as a separate program.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// How often the files are checked, in seconds: quick enough to feel instant.
const CHECK_EVERY: f32 = 0.25;
/// The shaders watched, in the folder given to `ShaderWatch::start`.
const VERTEX: &str = "mesh.vert";
const FRAGMENT: &str = "mesh.frag";

/// Watches `mesh.vert` and `mesh.frag` for saves.
#[derive(Debug)]
pub struct ShaderWatch {
    vertex: PathBuf,
    fragment: PathBuf,
    /// The modification times seen at the last check (vertex, fragment).
    last_seen: [Option<SystemTime>; 2],
    /// Seconds since the last check.
    since_check: f32,
}

impl ShaderWatch {
    /// Starts watching `folder`, or says why hot reload can't work here: the shader
    /// sources aren't there (scop run away from its repo), or `glslc` isn't installed.
    pub fn start(folder: &Path) -> Result<Self, String> {
        let vertex = folder.join(VERTEX);
        let fragment = folder.join(FRAGMENT);
        if !vertex.is_file() || !fragment.is_file() {
            return Err(format!("no shader sources in {}", folder.display()));
        }
        if !glslc_available() {
            return Err(String::from("glslc isn't installed (`make deps` says how)"));
        }
        let mut watch = ShaderWatch {
            vertex,
            fragment,
            last_seen: [None, None],
            since_check: 0.0,
        };
        // What's on disk now is the starting point: only *later* saves reload.
        watch.last_seen = watch.modification_times();
        Ok(watch)
    }

    /// One frame of `dt` seconds. Every `CHECK_EVERY` seconds, reads the files'
    /// modification times; `true` if either shader was saved since the last check.
    pub fn poll(&mut self, dt: f32) -> bool {
        self.since_check += dt;
        if self.since_check < CHECK_EVERY {
            return false;
        }
        self.since_check = 0.0;
        let now = self.modification_times();
        self.changed(now)
    }

    /// Compares `now` with what was seen last time, and remembers `now`. So each save
    /// is reported exactly once — even if compiling it then fails (the next save is a
    /// new change and gets its own try).
    fn changed(&mut self, now: [Option<SystemTime>; 2]) -> bool {
        let changed = now != self.last_seen;
        self.last_seen = now;
        changed
    }

    fn modification_times(&self) -> [Option<SystemTime>; 2] {
        let mtime = |path: &Path| fs::metadata(path).and_then(|m| m.modified()).ok();
        [mtime(&self.vertex), mtime(&self.fragment)]
    }

    /// Compiles both shaders: (vertex SPIR-V, fragment SPIR-V), or `glslc`'s error.
    pub fn compile_both(&self) -> Result<(Vec<u8>, Vec<u8>), String> {
        Ok((compile(&self.vertex)?, compile(&self.fragment)?))
    }
}

/// Runs `glslc <file> -o -`: SPIR-V comes back on its standard output, errors on its
/// standard error (with file and line, e.g. `mesh.frag:42: error: …`).
pub fn compile(path: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new("glslc")
        .arg(path)
        .args(["-o", "-"])
        .output()
        .map_err(|err| format!("could not run glslc: {err}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(String::from_utf8_lossy(&output.stderr)
            .trim_end()
            .to_string())
    }
}

/// Whether `glslc` can be run at all.
fn glslc_available() -> bool {
    Command::new("glslc")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// A watch on made-up files, for the change logic alone (no disk, no glslc).
    fn watch_with(last_seen: [Option<SystemTime>; 2]) -> ShaderWatch {
        ShaderWatch {
            vertex: PathBuf::from("v"),
            fragment: PathBuf::from("f"),
            last_seen,
            since_check: 0.0,
        }
    }

    #[test]
    fn a_save_is_noticed_exactly_once() {
        let t0 = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
        let t1 = t0 + Duration::from_secs(1);
        let mut watch = watch_with([Some(t0), Some(t0)]);
        assert!(!watch.changed([Some(t0), Some(t0)])); // nothing saved: no reload
        assert!(watch.changed([Some(t0), Some(t1)])); // mesh.frag saved
        assert!(!watch.changed([Some(t0), Some(t1)])); // same save: not again
    }

    #[test]
    fn checks_only_every_quarter_second() {
        let mut watch = watch_with([None, None]);
        assert!(!watch.poll(0.1));
        assert!((watch.since_check - 0.1).abs() < 1e-6); // not checked yet
        watch.poll(0.2); // 0.3 s ≥ 0.25: checks, and starts counting again
        assert_eq!(watch.since_check, 0.0);
    }

    #[test]
    fn missing_sources_switch_hot_reload_off() {
        let err = ShaderWatch::start(Path::new("definitely/not/here")).unwrap_err();
        assert!(err.contains("no shader sources"), "{err}");
    }

    #[test]
    fn a_broken_shader_returns_glslcs_error() {
        if !glslc_available() {
            return; // nothing to test without the compiler (e.g. on campus)
        }
        let dir = std::env::temp_dir().join(format!("scop-hot-reload-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let broken = dir.join("broken.frag");
        fs::write(&broken, "#version 450\nvoid main() { float x = 1.0 }\n").unwrap();
        let err = compile(&broken).unwrap_err();
        assert!(
            err.contains("broken.frag") && err.contains("error"),
            "{err}"
        );
        // And the real shaders compile to SPIR-V (it starts with the magic number).
        let spirv = compile(Path::new("shaders/mesh.frag")).unwrap();
        assert_eq!(spirv[..4], 0x0723_0203_u32.to_le_bytes());
        let _ = fs::remove_dir_all(&dir);
    }
}
