//! engine — reusable rendering engine for scop and future projects.

pub mod gpu;
pub mod platform;
mod swapchain;

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
