//! Our own SPIR-V loader: raw bytes → validated 32-bit words for Vulkan.
//! (The subject forbids libraries for shader loading, so no `ash::util::read_spv`.)

use std::fmt;

/// Every SPIR-V module starts with this word.
pub const MAGIC: u32 = 0x0723_0203;

/// Why some bytes aren't a usable SPIR-V module.
#[derive(Debug, PartialEq, Eq)]
pub enum SpirvError {
    /// SPIR-V is a list of 32-bit words, so the length must be a non-zero multiple of 4.
    BadLength(usize),
    /// The first word isn't `MAGIC` — probably not SPIR-V (e.g. GLSL text).
    BadMagic(u32),
}

impl fmt::Display for SpirvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpirvError::BadLength(n) => {
                write!(f, "SPIR-V size {n} is not a non-zero multiple of 4")
            }
            SpirvError::BadMagic(w) => write!(f, "invalid SPIR-V magic number {w:#010x}"),
        }
    }
}

impl std::error::Error for SpirvError {}

/// Turns SPIR-V bytes into words, checking the length and the magic number.
///
/// Copies into a new `Vec<u32>` rather than casting the slice: the bytes may not
/// start on a 4-byte boundary, and reading a misaligned `u32` is undefined behaviour.
pub fn words_from_bytes(bytes: &[u8]) -> Result<Vec<u32>, SpirvError> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
        return Err(SpirvError::BadLength(bytes.len()));
    }
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])) // lowest byte first
        .collect();
    if words[0] != MAGIC {
        return Err(SpirvError::BadMagic(words[0]));
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_little_endian_words() {
        let bytes = [0x03, 0x02, 0x23, 0x07, 0x01, 0x00, 0x00, 0x00];
        assert_eq!(words_from_bytes(&bytes), Ok(vec![MAGIC, 1]));
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(words_from_bytes(&[]), Err(SpirvError::BadLength(0)));
    }

    #[test]
    fn rejects_length_not_multiple_of_four() {
        let bytes = [0x03, 0x02, 0x23, 0x07, 0x01];
        assert_eq!(words_from_bytes(&bytes), Err(SpirvError::BadLength(5)));
    }

    #[test]
    fn rejects_glsl_text() {
        let text = b"#version 450\n";
        assert!(matches!(
            words_from_bytes(&text[..12]),
            Err(SpirvError::BadMagic(_))
        ));
    }
}
