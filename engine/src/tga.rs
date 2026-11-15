//! A hand-written TGA (Truevision Graphics Adapter, "Targa") loader (§ 14).
//!
//! Supported: image types 2 (uncompressed true-colour) and 10 (run-length encoded
//! true-colour), 24-bit (BGR) and 32-bit (BGRA), either row order. Everything else
//! (colour-mapped, greyscale, 16-bit) is refused with a clear `TgaError`.
//!
//! Output is always RGBA, 4 bytes per pixel, **top row first** — what the GPU upload
//! expects for UV (0, 0) = top-left.

use std::fmt;

/// A decoded picture: `width × height` pixels, RGBA, rows from top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// `width × height × 4` bytes: R, G, B, A for each pixel, row by row from the top.
    pub rgba: Vec<u8>,
}

/// Why a file couldn't be read as a TGA picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TgaError {
    /// Shorter than the 18-byte header (or than the header says the ID/colour map is).
    TooShort,
    /// An image type scop doesn't read (colour-mapped, greyscale, …).
    UnsupportedType(u8),
    /// Not 24 or 32 bits per pixel.
    UnsupportedDepth(u8),
    /// Width or height is 0.
    Empty,
    /// The pixel data ends before `width × height` pixels were read.
    Truncated,
}

impl fmt::Display for TgaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TgaError::TooShort => write!(f, "too short to be a TGA file"),
            TgaError::UnsupportedType(t) => write!(
                f,
                "TGA image type {t} not supported (only 2 = true-colour, 10 = RLE true-colour)"
            ),
            TgaError::UnsupportedDepth(b) => {
                write!(f, "{b} bits per pixel not supported (only 24 or 32)")
            }
            TgaError::Empty => write!(f, "image has zero width or height"),
            TgaError::Truncated => write!(f, "pixel data ends early (file cut short?)"),
        }
    }
}

impl std::error::Error for TgaError {}

const HEADER_SIZE: usize = 18;
/// Descriptor bit 5: set = the first stored row is the picture's top.
const TOP_FIRST: u8 = 0x20;

/// Decodes the bytes of a `.tga` file.
pub fn load_tga(bytes: &[u8]) -> Result<Image, TgaError> {
    let header = bytes.get(..HEADER_SIZE).ok_or(TgaError::TooShort)?;
    let id_length = usize::from(header[0]);
    let colour_map_type = header[1];
    let image_type = header[2];
    let width = u16::from_le_bytes([header[12], header[13]]);
    let height = u16::from_le_bytes([header[14], header[15]]);
    let bits = header[16];
    let descriptor = header[17];

    let compressed = match (image_type, colour_map_type) {
        (2, 0) => false,
        (10, 0) => true,
        _ => return Err(TgaError::UnsupportedType(image_type)),
    };
    let bytes_per_pixel = match bits {
        24 => 3,
        32 => 4,
        other => return Err(TgaError::UnsupportedDepth(other)),
    };
    if width == 0 || height == 0 {
        return Err(TgaError::Empty);
    }

    // Skip the optional image ID (a free-text label) after the header.
    let data = bytes
        .get(HEADER_SIZE + id_length..)
        .ok_or(TgaError::TooShort)?;
    let pixel_count = usize::from(width) * usize::from(height);
    let stored = if compressed {
        decode_rle(data, pixel_count, bytes_per_pixel)?
    } else {
        let len = pixel_count * bytes_per_pixel;
        data.get(..len).ok_or(TgaError::Truncated)?.to_vec()
    };

    // BGR(A) → RGBA. Each stored pixel becomes 4 output bytes.
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for px in stored.chunks_exact(bytes_per_pixel) {
        let alpha = if bytes_per_pixel == 4 { px[3] } else { 255 };
        rgba.extend_from_slice(&[px[2], px[1], px[0], alpha]);
    }

    // TGA's default row order is bottom-up: flip so row 0 is the top.
    if descriptor & TOP_FIRST == 0 {
        flip_rows(&mut rgba, usize::from(width) * 4);
    }

    Ok(Image {
        width: u32::from(width),
        height: u32::from(height),
        rgba,
    })
}

/// Run-length decoding (type 10). Each packet starts with one byte `h`:
/// - `h & 0x80` set → one pixel follows, repeated `(h & 0x7F) + 1` times;
/// - otherwise     → `(h & 0x7F) + 1` different pixels follow.
fn decode_rle(data: &[u8], pixel_count: usize, bpp: usize) -> Result<Vec<u8>, TgaError> {
    let mut out = Vec::with_capacity(pixel_count * bpp);
    let mut pos = 0;
    while out.len() < pixel_count * bpp {
        let header = *data.get(pos).ok_or(TgaError::Truncated)?;
        pos += 1;
        let count = usize::from(header & 0x7F) + 1;
        if header & 0x80 != 0 {
            let pixel = data.get(pos..pos + bpp).ok_or(TgaError::Truncated)?;
            pos += bpp;
            for _ in 0..count {
                out.extend_from_slice(pixel);
            }
        } else {
            let pixels = data
                .get(pos..pos + count * bpp)
                .ok_or(TgaError::Truncated)?;
            pos += count * bpp;
            out.extend_from_slice(pixels);
        }
    }
    // A packet may run past the last pixel in a broken file: keep exactly the image.
    out.truncate(pixel_count * bpp);
    Ok(out)
}

/// Reverses the order of the rows (each `row_len` bytes long), in place.
fn flip_rows(pixels: &mut [u8], row_len: usize) {
    let rows = pixels.len() / row_len;
    for top in 0..rows / 2 {
        let bottom = rows - 1 - top;
        let (upper, lower) = pixels.split_at_mut(bottom * row_len);
        upper[top * row_len..(top + 1) * row_len].swap_with_slice(&mut lower[..row_len]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An 18-byte header for a `w × h` image.
    fn header(image_type: u8, w: u16, h: u16, bits: u8, descriptor: u8) -> Vec<u8> {
        let mut h_ = vec![0u8; HEADER_SIZE];
        h_[2] = image_type;
        h_[12..14].copy_from_slice(&w.to_le_bytes());
        h_[14..16].copy_from_slice(&h.to_le_bytes());
        h_[16] = bits;
        h_[17] = descriptor;
        h_
    }

    const RED: [u8; 4] = [255, 0, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];

    #[test]
    fn bgr_becomes_rgba_and_bottom_up_rows_are_flipped() {
        // 1×2, 24-bit, bottom row first: stored [blue (BGR), red (BGR)].
        let mut file = header(2, 1, 2, 24, 0);
        file.extend_from_slice(&[255, 0, 0]); // blue, stored first = bottom
        file.extend_from_slice(&[0, 0, 255]); // red, stored second = top
        let img = load_tga(&file).unwrap();
        assert_eq!((img.width, img.height), (1, 2));
        assert_eq!(&img.rgba[..4], &RED); // top row
        assert_eq!(&img.rgba[4..], &BLUE); // bottom row
    }

    #[test]
    fn top_first_rows_are_kept_and_alpha_is_read() {
        let mut file = header(2, 1, 2, 32, TOP_FIRST);
        file.extend_from_slice(&[0, 0, 255, 128]); // red, half transparent (BGRA)
        file.extend_from_slice(&[255, 0, 0, 255]); // blue
        let img = load_tga(&file).unwrap();
        assert_eq!(&img.rgba[..4], &[255, 0, 0, 128]);
        assert_eq!(&img.rgba[4..], &BLUE);
    }

    #[test]
    fn image_id_is_skipped() {
        let mut file = header(2, 1, 1, 24, 0);
        file[0] = 3; // 3-byte ID follows the header
        file.extend_from_slice(b"abc");
        file.extend_from_slice(&[255, 0, 0]);
        assert_eq!(load_tga(&file).unwrap().rgba, BLUE.to_vec());
    }

    #[test]
    fn rle_repeat_and_raw_packets() {
        // 4×1 RLE: 3× red (repeat packet), then 1 raw blue.
        let mut file = header(10, 4, 1, 24, TOP_FIRST);
        file.extend_from_slice(&[0x80 | 2, 0, 0, 255]); // repeat 3 times: red
        file.extend_from_slice(&[0, 255, 0, 0]); // 1 raw pixel: blue
        let img = load_tga(&file).unwrap();
        assert_eq!(img.rgba, [RED, RED, RED, BLUE].concat());
    }

    #[test]
    fn broken_files_give_errors() {
        assert_eq!(load_tga(&[0; 5]), Err(TgaError::TooShort));
        assert_eq!(
            load_tga(&header(1, 1, 1, 24, 0)),
            Err(TgaError::UnsupportedType(1))
        );
        assert_eq!(
            load_tga(&header(2, 1, 1, 16, 0)),
            Err(TgaError::UnsupportedDepth(16))
        );
        assert_eq!(load_tga(&header(2, 0, 1, 24, 0)), Err(TgaError::Empty));
        assert_eq!(load_tga(&header(2, 2, 2, 24, 0)), Err(TgaError::Truncated));
        assert_eq!(load_tga(&header(10, 2, 2, 24, 0)), Err(TgaError::Truncated));
    }
}
