//! Text on screen (§ 25 checkpoint extra, owner's request): a built-in bitmap font
//! and the layout that turns a string into glyph rectangles.
//!
//! The font is X11's `misc-fixed` 8×13 ("Public domain font. Share and enjoy."),
//! rasterised once into `assets/font_8x13.tga`: 16 × 6 cells for ASCII 32..=127, cell
//! 127 filled solid (used for panels). Read with our own TGA loader (§ 14) — no font
//! library (only window and event libraries are allowed).
//!
//! Drawing uses the instance list (§ 22): one unit quad, one instance per glyph; the
//! instance's "matrix" carries the glyph's rectangle, its cell in the font, its colour.

use crate::maths::Mat4;

/// The font picture, embedded in the engine.
pub(crate) const FONT_TGA: &[u8] = include_bytes!("../assets/font_8x13.tga");
/// One glyph's size in the font, in texels (and on screen at scale 1, in pixels).
pub const GLYPH_WIDTH: u32 = 8;
pub const GLYPH_HEIGHT: u32 = 13;
/// Cells per row of the font picture.
const COLUMNS: u32 = 16;
/// The solid cell (ASCII 127): every texel filled, for panels behind text.
const SOLID: u8 = 127;

/// One glyph to draw: where (pixels, from the top-left of the window) and which cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Glyph {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) character: u8,
}

/// Lays `text` out from (x, y), `scale` screen pixels per font texel: left to right,
/// `\n` starts a new line. Spaces take room but draw nothing; anything that isn't
/// printable ASCII is drawn as `?`.
pub(crate) fn layout_text(text: &str, (x, y): (f32, f32), scale: f32) -> Vec<Glyph> {
    let (w, h) = (GLYPH_WIDTH as f32 * scale, GLYPH_HEIGHT as f32 * scale);
    let mut glyphs = Vec::new();
    for (row, line) in text.split('\n').enumerate() {
        for (column, c) in line.chars().enumerate() {
            let character = match c {
                ' ' => continue,
                '!'..='~' => c as u8,
                _ => b'?',
            };
            glyphs.push(Glyph {
                x: x + column as f32 * w,
                y: y + row as f32 * h,
                width: w,
                height: h,
                character,
            });
        }
    }
    glyphs
}

/// How much room `text` takes at `scale`, in pixels: (widest line, all lines).
pub fn text_size(text: &str, scale: f32) -> (f32, f32) {
    let lines = text.split('\n');
    let (columns, rows) = lines.fold((0, 0), |(widest, rows), line| {
        (widest.max(line.chars().count()), rows + 1)
    });
    (
        columns as f32 * GLYPH_WIDTH as f32 * scale,
        rows as f32 * GLYPH_HEIGHT as f32 * scale,
    )
}

/// A solid rectangle, as a "glyph" using the solid cell.
pub(crate) fn panel(x: f32, y: f32, width: f32, height: f32) -> Glyph {
    Glyph {
        x,
        y,
        width,
        height,
        character: SOLID,
    }
}

/// The instance "matrix" the text shader reads: column 0 = rectangle (x, y, w, h) in
/// pixels, column 1 = the cell in the font (x, y, w, h) in texels, column 2 = colour.
pub(crate) fn glyph_instance(glyph: &Glyph, colour: [f32; 3]) -> Mat4 {
    let cell = u32::from(glyph.character - b' ');
    let (cell_x, cell_y) = (
        (cell % COLUMNS) * GLYPH_WIDTH,
        (cell / COLUMNS) * GLYPH_HEIGHT,
    );
    let columns = [
        [glyph.x, glyph.y, glyph.width, glyph.height],
        [
            cell_x as f32,
            cell_y as f32,
            GLYPH_WIDTH as f32,
            GLYPH_HEIGHT as f32,
        ],
        [colour[0], colour[1], colour[2], 1.0],
        [0.0; 4],
    ];
    // `from_rows` takes rows: row r = the r-th number of each column.
    Mat4::from_rows(std::array::from_fn(|r| columns.map(|c| c[r])))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tga::load_tga;

    #[test]
    fn lines_and_columns() {
        let glyphs = layout_text("ab\n c", (10.0, 20.0), 2.0);
        let places: Vec<_> = glyphs.iter().map(|g| (g.character, g.x, g.y)).collect();
        // 16 × 26 pixels per glyph at scale 2; the space takes room but isn't drawn.
        assert_eq!(
            places,
            vec![(b'a', 10.0, 20.0), (b'b', 26.0, 20.0), (b'c', 26.0, 46.0)]
        );
        assert_eq!(layout_text("→", (0.0, 0.0), 1.0)[0].character, b'?');
        assert_eq!(text_size("ab\n cde", 1.0), (32.0, 26.0));
    }

    #[test]
    fn every_printable_character_has_a_glyph() {
        let font = load_tga(FONT_TGA).unwrap();
        assert_eq!(
            (font.width, font.height),
            (COLUMNS * GLYPH_WIDTH, 6 * GLYPH_HEIGHT)
        );
        let alpha = |x: u32, y: u32| font.rgba[((y * font.width + x) * 4 + 3) as usize];
        let lit = |c: u8| {
            let cell = u32::from(c - b' ');
            let (x0, y0) = (
                (cell % COLUMNS) * GLYPH_WIDTH,
                (cell / COLUMNS) * GLYPH_HEIGHT,
            );
            (0..GLYPH_HEIGHT)
                .flat_map(|y| (0..GLYPH_WIDTH).map(move |x| (x, y)))
                .filter(|&(x, y)| alpha(x0 + x, y0 + y) > 127)
                .count()
        };
        assert_eq!(lit(b' '), 0);
        for c in b'!'..=b'~' {
            assert!(lit(c) > 0, "no glyph for {:?}", c as char);
        }
        assert_eq!(lit(SOLID), (GLYPH_WIDTH * GLYPH_HEIGHT) as usize); // fully solid
    }

    #[test]
    fn instances_carry_rectangle_cell_and_colour() {
        let g = layout_text("A", (5.0, 6.0), 1.0)[0];
        let cols = glyph_instance(&g, [0.1, 0.2, 0.3]).to_cols_array();
        // 'A' = 65 → cell 33 → column 1, row 2 of the font.
        assert_eq!(cols[0], [5.0, 6.0, 8.0, 13.0]);
        assert_eq!(cols[1], [8.0, 26.0, 8.0, 13.0]);
        assert_eq!(cols[2], [0.1, 0.2, 0.3, 1.0]);
    }
}
