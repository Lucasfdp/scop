//! Reading Wavefront `.obj` files (§ 12): corners (`v`) and faces (`f`), nothing else.
//!
//! - Corners are numbered from 1 in the file; the output is 0-based, ready for the GPU.
//! - Faces with more than 3 corners are fan-triangulated: `a b c d` → `a b c`, `a c d`
//!   (correct for convex faces; concave ones are the § 17 bonus).
//! - Every other line (`#`, `o`, `g`, `s`, `mtllib`, `usemtl`, `vt`, `vn`, `l`, …) is skipped.
//! - A broken file gives an `ObjError` with its line number, never a panic.

use std::fmt;

use crate::maths::Vec3;

/// A parsed model: corner positions, triangles (0-based corner indices) and the
/// bounding box around every corner.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjModel {
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    /// For each triangle, which face line (counted from 0) it came from: a quad's two
    /// triangles share one number, so they can share one colour (§ 13).
    pub triangle_faces: Vec<u32>,
    /// Smallest x, y and z of any corner.
    pub min: Vec3,
    /// Largest x, y and z of any corner.
    pub max: Vec3,
}

impl ObjModel {
    /// The middle of the bounding box: what the model spins around.
    pub fn centre(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Half the bounding box's diagonal: a sphere this big around `centre` holds
    /// the whole model. Used to place the camera far enough back.
    pub fn radius(&self) -> f32 {
        (self.max - self.min).length() * 0.5
    }
}

/// Why a file couldn't be read as a model.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjError {
    /// 1-based line number, as an editor shows it. 0 = about the whole file.
    pub line: usize,
    pub kind: ObjErrorKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjErrorKind {
    /// A `v` line with fewer than 3 numbers.
    MissingCoordinate,
    /// Something that should be a number isn't (or is NaN / infinite).
    BadNumber(String),
    /// A face uses corner 0, which doesn't exist (`.obj` counts from 1).
    ZeroIndex,
    /// A face uses a corner that hasn't been defined (yet).
    IndexOutOfRange { index: i64, defined: usize },
    /// A face with fewer than 3 corners.
    TooFewCorners(usize),
    /// The file has no faces at all: nothing to draw.
    NoFaces,
}

impl fmt::Display for ObjError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: ", self.line)?;
        }
        match &self.kind {
            ObjErrorKind::MissingCoordinate => write!(f, "a corner needs 3 numbers (x y z)"),
            ObjErrorKind::BadNumber(token) => write!(f, "'{token}' is not a valid number"),
            ObjErrorKind::ZeroIndex => write!(f, "corner 0 doesn't exist (.obj counts from 1)"),
            ObjErrorKind::IndexOutOfRange { index, defined } => write!(
                f,
                "face uses corner {index}, but only {defined} corners are defined so far"
            ),
            ObjErrorKind::TooFewCorners(n) => {
                write!(f, "a face needs at least 3 corners, this one has {n}")
            }
            ObjErrorKind::NoFaces => write!(f, "no faces: nothing to draw"),
        }
    }
}

impl std::error::Error for ObjError {}

/// Parses the text of an `.obj` file.
pub fn parse_obj(text: &str) -> Result<ObjModel, ObjError> {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let mut triangle_faces: Vec<u32> = Vec::new();
    let mut face_count: u32 = 0;

    // `lines()` also strips Windows "\r\n" endings. Editors count lines from 1.
    for (number, line) in text.lines().enumerate().map(|(i, l)| (i + 1, l)) {
        let error = |kind| ObjError { line: number, kind };
        // Everything after '#' is a comment.
        let content = line.split('#').next().unwrap_or("");
        let mut words = content.split_whitespace();
        match words.next() {
            Some("v") => positions.push(parse_corner(words).map_err(error)?),
            Some("f") => {
                let corners: Vec<u32> = words
                    .map(|token| corner_index(token, positions.len()))
                    .collect::<Result<_, _>>()
                    .map_err(error)?;
                if corners.len() < 3 {
                    return Err(error(ObjErrorKind::TooFewCorners(corners.len())));
                }
                // Fan from the first corner: (c0, c1, c2), (c0, c2, c3), …
                for pair in corners[1..].windows(2) {
                    triangles.push([corners[0], pair[0], pair[1]]);
                    triangle_faces.push(face_count);
                }
                face_count += 1;
            }
            _ => {} // blank, comment, or a line type scop doesn't need
        }
    }

    if triangles.is_empty() {
        return Err(ObjError {
            line: 0,
            kind: ObjErrorKind::NoFaces,
        });
    }
    let (min, max) = bounding_box(&positions);
    Ok(ObjModel {
        positions,
        triangles,
        triangle_faces,
        min,
        max,
    })
}

/// The numbers after `v`: x y z (an optional 4th, `w`, is ignored).
fn parse_corner<'a>(mut words: impl Iterator<Item = &'a str>) -> Result<Vec3, ObjErrorKind> {
    let mut next = || {
        let token = words.next().ok_or(ObjErrorKind::MissingCoordinate)?;
        match token.parse::<f32>() {
            Ok(value) if value.is_finite() => Ok(value),
            _ => Err(ObjErrorKind::BadNumber(token.to_string())),
        }
    };
    Ok(Vec3::new(next()?, next()?, next()?))
}

/// One face token (`5`, `5/2`, `5/2/7`, `5//7`, `-1`) → a 0-based corner index,
/// checked against the `defined` corners read so far.
fn corner_index(token: &str, defined: usize) -> Result<u32, ObjErrorKind> {
    // Only the part before the first '/' is the corner.
    let corner = token.split('/').next().unwrap_or(token);
    let n: i64 = corner
        .parse()
        .map_err(|_| ObjErrorKind::BadNumber(token.to_string()))?;
    let out_of_range = ObjErrorKind::IndexOutOfRange { index: n, defined };
    let index = match n {
        0 => return Err(ObjErrorKind::ZeroIndex),
        n if n > 0 => n - 1,     // 1-based → 0-based
        n => defined as i64 + n, // negative: −1 = the last corner so far
    };
    if index < 0 || index >= defined as i64 {
        return Err(out_of_range);
    }
    u32::try_from(index).map_err(|_| out_of_range)
}

/// Smallest and largest x, y, z over all corners.
fn bounding_box(positions: &[Vec3]) -> (Vec3, Vec3) {
    // Faces exist (checked by the caller), so there is at least one corner.
    let first = positions[0];
    positions.iter().fold((first, first), |(min, max), p| {
        (
            Vec3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z)),
            Vec3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z)),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\n";

    fn kind_of(text: &str) -> ObjErrorKind {
        parse_obj(text).unwrap_err().kind
    }

    #[test]
    fn triangle_becomes_zero_based() {
        let model = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        assert_eq!(model.positions.len(), 3);
        assert_eq!(model.triangles, vec![[0, 1, 2]]);
    }

    #[test]
    fn quad_is_fanned_from_its_first_corner() {
        // Q-12.2's pattern: f a b c d → (a b c), (a c d), then − 1.
        let model = parse_obj(&format!("{SQUARE}f 4 1 2 3\n")).unwrap();
        assert_eq!(model.triangles, vec![[3, 0, 1], [3, 1, 2]]);
    }

    #[test]
    fn a_quads_two_triangles_share_one_face_number() {
        let model = parse_obj(&format!("{SQUARE}f 1 2 3\nf 1 2 3 4\nf 2 3 4\n")).unwrap();
        // face 0: one triangle; face 1: a quad → two triangles; face 2: one triangle.
        assert_eq!(model.triangle_faces, vec![0, 1, 1, 2]);
        assert_eq!(model.triangle_faces.len(), model.triangles.len());
    }

    #[test]
    fn slash_tokens_use_the_corner_part() {
        let model = parse_obj(&format!("{SQUARE}f 1/1/1 2//2 3/7\n")).unwrap();
        assert_eq!(model.triangles, vec![[0, 1, 2]]);
    }

    #[test]
    fn negative_indices_count_back_from_the_last_corner() {
        let model = parse_obj(&format!("{SQUARE}f -4 -3 -2 -1\n")).unwrap();
        assert_eq!(model.triangles, vec![[0, 1, 2], [0, 2, 3]]);
    }

    #[test]
    fn comments_blank_and_unknown_lines_are_skipped() {
        let text = "# hi\n\nmtllib x.mtl\no Cube\nv 0 0 0 # end comment\nv 1 0 0\r\nv 0 1 0\nvt 0 0\nvn 0 0 1\ns off\nusemtl M\nf 1 2 3\n";
        assert_eq!(parse_obj(text).unwrap().triangles, vec![[0, 1, 2]]);
    }

    #[test]
    fn bounding_box_centre_and_radius() {
        let model = parse_obj("v 0 0 0\nv 2 4 0\nv 0 0 4\nf 1 2 3\n").unwrap();
        assert_eq!(model.min, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(model.max, Vec3::new(2.0, 4.0, 4.0));
        assert_eq!(model.centre(), Vec3::new(1.0, 2.0, 2.0));
        assert!((model.radius() - 3.0).abs() < 1e-5); // diagonal 6, half = 3
    }

    #[test]
    fn bad_files_give_errors_with_line_numbers() {
        assert_eq!(
            kind_of("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 0 1 2\n"),
            ObjErrorKind::ZeroIndex
        );
        let err = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 4\n").unwrap_err();
        assert_eq!(err.line, 4);
        assert_eq!(
            err.kind,
            ObjErrorKind::IndexOutOfRange {
                index: 4,
                defined: 3
            }
        );
        assert_eq!(
            kind_of(&format!("{SQUARE}f 1 2\n")),
            ObjErrorKind::TooFewCorners(2)
        );
        assert_eq!(kind_of("v 0 0\n"), ObjErrorKind::MissingCoordinate);
        assert_eq!(kind_of("v 0 x 0\n"), ObjErrorKind::BadNumber("x".into()));
        assert_eq!(
            kind_of("v nan 0 0\n"),
            ObjErrorKind::BadNumber("nan".into())
        );
        assert_eq!(kind_of(SQUARE), ObjErrorKind::NoFaces);
        // A face may only use corners defined *before* it.
        assert!(matches!(
            kind_of("f 1 2 3\nv 0 0 0\n"),
            ObjErrorKind::IndexOutOfRange { .. }
        ));
    }

    #[test]
    fn error_message_reads_well() {
        let err = parse_obj("v 0 0 0\nf 1 2 3\n").unwrap_err();
        assert_eq!(
            err.to_string(),
            "line 2: face uses corner 2, but only 1 corners are defined so far"
        );
    }

    #[test]
    fn real_files_load() {
        let logo = parse_obj(&std::fs::read_to_string("../resources/42.obj").unwrap()).unwrap();
        assert_eq!(logo.positions.len(), 42);
        assert_eq!(logo.triangles.len(), 18 + 29 * 2); // 18 triangles + 29 quads × 2
        let teapot =
            parse_obj(&std::fs::read_to_string("../resources/teapot.obj").unwrap()).unwrap();
        assert_eq!(teapot.positions.len(), 3644);
        assert_eq!(teapot.triangles.len(), 6320);
    }
}
