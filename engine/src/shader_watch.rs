//! Shader hot reload (§ 21 — moved into the engine in § 24): notice a saved shader,
//! compile it with `glslc`, hand the SPIR-V back.
//!
//! The split (Q-24.4): the engine notices and compiles; the **app** decides which of
//! its pipelines get the new shaders (`Gfx::replace_pipeline`), because only the app
//! knows which pipelines were built from which files, in which style.
//!
//! Only `std`: polling modification times (no file-watching library — the subject only
//! allows window and event libraries) and running `glslc` as a separate program.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// How often the files are checked, in seconds: quick enough to feel instant.
const CHECK_EVERY: f32 = 0.25;

/// Watches a list of shader source files for saves.
#[derive(Debug)]
pub struct ShaderWatch {
    files: Vec<PathBuf>,
    /// The modification times seen at the last check, one per file.
    last_seen: Vec<Option<SystemTime>>,
    /// Seconds since the last check.
    since_check: f32,
}

impl ShaderWatch {
    /// Starts watching `files`, or says why hot reload can't work here: a source isn't
    /// there (the program runs away from its repo), or `glslc` isn't installed.
    pub fn start(files: &[PathBuf]) -> Result<Self, String> {
        if let Some(missing) = files.iter().find(|f| !f.is_file()) {
            return Err(format!("no shader source at {}", missing.display()));
        }
        if !glslc_available() {
            return Err(String::from("glslc isn't installed (`make deps` says how)"));
        }
        let mut watch = ShaderWatch {
            files: files.to_vec(),
            last_seen: Vec::new(),
            since_check: 0.0,
        };
        // What's on disk now is the starting point: only *later* saves reload.
        watch.last_seen = watch.modification_times();
        Ok(watch)
    }

    /// One frame of `dt` seconds. Every quarter-second, reads the files' modification
    /// times; `true` if any was saved since the last check.
    pub fn poll(&mut self, dt: f32) -> bool {
        self.since_check += dt;
        if self.since_check < CHECK_EVERY {
            return false;
        }
        self.since_check = 0.0;
        let now = self.modification_times();
        self.changed(now)
    }

    /// Compares `now` with what was seen last time, and remembers `now`: each save is
    /// reported exactly once, even if compiling it then fails.
    fn changed(&mut self, now: Vec<Option<SystemTime>>) -> bool {
        let changed = now != self.last_seen;
        self.last_seen = now;
        changed
    }

    fn modification_times(&self) -> Vec<Option<SystemTime>> {
        let mtime = |path: &PathBuf| fs::metadata(path).and_then(|m| m.modified()).ok();
        self.files.iter().map(mtime).collect()
    }

    /// Compiles every watched file, in the order given to `start`, or returns the
    /// first `glslc` error.
    pub fn compile_all(&self) -> Result<Vec<Vec<u8>>, String> {
        self.files.iter().map(|f| compile_shader(f)).collect()
    }
}

/// Runs `glslc <file> -o -`: SPIR-V comes back on its standard output, errors on its
/// standard error (with file and line, e.g. `default.frag:42: error: …`).
pub fn compile_shader(path: &Path) -> Result<Vec<u8>, String> {
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

    fn watch_with(last_seen: Vec<Option<SystemTime>>) -> ShaderWatch {
        ShaderWatch {
            files: vec![PathBuf::from("a"); last_seen.len()],
            last_seen,
            since_check: 0.0,
        }
    }

    #[test]
    fn a_save_is_noticed_exactly_once() {
        let t0 = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
        let t1 = t0 + Duration::from_secs(1);
        let mut watch = watch_with(vec![Some(t0); 4]);
        assert!(!watch.changed(vec![Some(t0); 4])); // nothing saved: no reload
        let saved = vec![Some(t0), Some(t1), Some(t0), Some(t0)];
        assert!(watch.changed(saved.clone())); // one file saved
        assert!(!watch.changed(saved)); // same save: not again
    }

    #[test]
    fn checks_only_every_quarter_second() {
        let mut watch = watch_with(vec![None]);
        assert!(!watch.poll(0.1));
        assert!((watch.since_check - 0.1).abs() < 1e-6); // not checked yet
        watch.poll(0.2); // 0.3 s ≥ 0.25: checks, and starts counting again
        assert_eq!(watch.since_check, 0.0);
    }

    #[test]
    fn a_missing_source_switches_hot_reload_off() {
        let err = ShaderWatch::start(&[PathBuf::from("definitely/not/here.frag")]).unwrap_err();
        assert!(err.contains("no shader source"), "{err}");
    }

    #[test]
    fn a_broken_shader_returns_glslcs_error() {
        if !glslc_available() {
            return; // nothing to test without the compiler (e.g. on campus)
        }
        let dir = std::env::temp_dir().join(format!("engine-shader-watch-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let broken = dir.join("broken.frag");
        fs::write(&broken, "#version 450\nvoid main() { float x = 1.0 }\n").unwrap();
        let err = compile_shader(&broken).unwrap_err();
        assert!(
            err.contains("broken.frag") && err.contains("error"),
            "{err}"
        );
        // The engine's own default shader compiles to SPIR-V (magic number first).
        let spirv = compile_shader(Path::new("shaders/default.frag")).unwrap();
        assert_eq!(spirv[..4], 0x0723_0203_u32.to_le_bytes());
        let _ = fs::remove_dir_all(&dir);
    }
}
