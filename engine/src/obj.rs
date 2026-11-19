//! Reading Wavefront `.obj` files (§ 12): corners (`v`), faces (`f`) and, since § 18,
//! object boundaries (`o` / `g`).
//!
//! - Corners are numbered from 1 in the file; the output is 0-based, ready for the GPU.
//! - Faces with more than 3 corners are split by ear clipping (§ 17,
//!   `triangulate.rs`): correct for concave and bent faces too.
//! - `o name` and `g name` start a new object (§ 18); a file without them is one object.
//! - Every other line (`#`, `s`, `mtllib`, `usemtl`, `vt`, `vn`, `l`, …) is skipped.
//! - A broken file gives an `ObjError` with its line number, never a panic.

use std::fmt;

use crate::maths::Vec3;
use crate::triangulate::triangulate;

/// A parsed model: corner positions, triangles (0-based corner indices) and the
/// bounding box around every corner.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjModel {
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    /// For each triangle, which face line (counted from 0) it came from: a quad's two
    /// triangles share one number, so they can share one colour (§ 13).
    pub triangle_faces: Vec<u32>,
    /// For each triangle, which object (counted from 0) it belongs to (§ 18).
    pub triangle_objects: Vec<u32>,
    /// One name per object, from its `o` / `g` line; `"default"` when the file has none.
    /// Every object has at least one triangle.
    pub object_names: Vec<String>,
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

    /// How many edges belong to only one triangle: borders where the surface is open
    /// and you can see past it (§ 17). A sealed model has 0. Corners at exactly the same
    /// position count as one, so seams between separately modelled parts don't count.
    pub fn open_edges(&self) -> usize {
        use std::collections::HashMap;
        // `+ 0.0` turns −0.0 into 0.0: the same point, but a different bit pattern
        // (teapot.obj writes 86 coordinates as "-0.000000").
        let bits = |v: f32| (v + 0.0).to_bits();
        let key = |i: u32| {
            let p = self.positions[i as usize];
            [bits(p.x), bits(p.y), bits(p.z)]
        };
        let mut uses: HashMap<_, usize> = HashMap::new();
        for t in &self.triangles {
            for k in 0..3 {
                let (a, b) = (key(t[k]), key(t[(k + 1) % 3]));
                let edge = if a <= b { (a, b) } else { (b, a) };
                *uses.entry(edge).or_default() += 1;
            }
        }
        uses.values().filter(|&&n| n == 1).count()
    }

    /// The bounding box of only `object`'s corners (§ 18: each object's picture is
    /// fitted to its own box). `None` if there's no such object.
    pub fn object_box(&self, object: u32) -> Option<(Vec3, Vec3)> {
        let mut corners = self
            .triangles
            .iter()
            .zip(&self.triangle_objects)
            .filter(|&(_, &o)| o == object)
            .flat_map(|(t, _)| t.iter().map(|&c| self.positions[c as usize]));
        let first = corners.next()?;
        Some(corners.fold((first, first), grow_box))
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
    let mut triangle_objects: Vec<u32> = Vec::new();
    // Object 0 exists from the start, so faces before any `o` line have a home.
    let mut object_names: Vec<String> = vec![String::from("default")];

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
                // Ear clipping (§ 17); plain triangles come back as they are.
                let object = (object_names.len() - 1) as u32; // the latest one
                for triangle in triangulate(&positions, &corners) {
                    triangles.push(triangle);
                    triangle_faces.push(face_count);
                    triangle_objects.push(object);
                }
                face_count += 1;
            }
            Some("o" | "g") => {
                // The rest of the line is the name (names may contain spaces).
                let name = words.collect::<Vec<_>>().join(" ");
                let name = if name.is_empty() {
                    String::from("unnamed")
                } else {
                    name
                };
                // The current object has no faces yet (e.g. `o` right at the top, or
                // `o Cube` followed by `g sides`): rename it instead of leaving it empty.
                let current_is_empty =
                    triangle_objects.last() != Some(&((object_names.len() - 1) as u32));
                if current_is_empty {
                    *object_names.last_mut().expect("object 0 always exists") = name;
                } else {
                    object_names.push(name);
                }
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
    // An `o` line after the last face would leave an object with nothing in it.
    if triangle_objects.last() != Some(&((object_names.len() - 1) as u32)) {
        object_names.pop();
    }
    let (min, max) = bounding_box(&positions);
    Ok(ObjModel {
        positions,
        triangles,
        triangle_faces,
        triangle_objects,
        object_names,
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
    positions.iter().copied().fold((first, first), grow_box)
}

/// A box, grown just enough to also hold `p`.
fn grow_box((min, max): (Vec3, Vec3), p: Vec3) -> (Vec3, Vec3) {
    (
        Vec3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z)),
        Vec3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z)),
    )
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
    fn quad_becomes_two_triangles() {
        // Since § 17 ear clipping; on a convex face it gives exactly § 12's fan:
        // (a b c), (a c d), then − 1.
        let model = parse_obj(&format!("{SQUARE}f 4 1 2 3\n")).unwrap();
        assert_eq!(model.triangles, vec![[3, 0, 1], [3, 1, 2]]);
    }

    #[test]
    fn open_edges_counted() {
        // A single square: all 4 outline edges are open (the diagonal is shared).
        let model = parse_obj(&format!("{SQUARE}f 1 2 3 4\n")).unwrap();
        assert_eq!(model.open_edges(), 4);
        // −0 and 0 are the same corner: these two triangles share an edge.
        let signed_zero =
            parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nv -0 1 0\nv 1 1 0\nf 1 2 3\nf 4 2 5\n").unwrap();
        assert_eq!(signed_zero.open_edges(), 4);
        // A closed cube: none.
        let read = |n: &str| std::fs::read_to_string(format!("../resources/test/{n}")).unwrap();
        assert_eq!(parse_obj(&read("cube_vt_vn.obj")).unwrap().open_edges(), 0);
    }

    #[test]
    fn a_quads_two_triangles_share_one_face_number() {
        let model = parse_obj(&format!("{SQUARE}f 1 2 3\nf 1 2 3 4\nf 2 3 4\n")).unwrap();
        // face 0: one triangle; face 1: a quad → two triangles; face 2: one triangle.
        assert_eq!(model.triangle_faces, vec![0, 1, 1, 2]);
        assert_eq!(model.triangle_faces.len(), model.triangles.len());
    }

    #[test]
    fn o_and_g_lines_split_objects() {
        // No o/g lines: one object, called "default".
        let one = parse_obj(&format!("{SQUARE}f 1 2 3\nf 1 3 4\n")).unwrap();
        assert_eq!(one.object_names, vec!["default"]);
        assert_eq!(one.triangle_objects, vec![0, 0]);
        // `o` before any face renames object 0; `g` after faces starts object 1;
        // `o Cube` straight after `g` (no faces between) renames it; a trailing `o`
        // with nothing after it is dropped.
        let text = format!("o first\n{SQUARE}f 1 2 3\ng\no second one\nf 1 2 3 4\no empty\n");
        let model = parse_obj(&text).unwrap();
        assert_eq!(model.object_names, vec!["first", "second one"]);
        assert_eq!(model.triangle_objects, vec![0, 1, 1]); // the quad's 2 triangles
        assert_eq!(model.triangle_objects.len(), model.triangles.len());
    }

    #[test]
    fn object_box_covers_only_that_object() {
        let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 5 5 5\nv 6 5 5\nv 5 7 5\n\
                    o a\nf 1 2 3\no b\nf 4 5 6\n";
        let model = parse_obj(text).unwrap();
        assert_eq!(
            model.object_box(0),
            Some((Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0)))
        );
        let b = (Vec3::new(5.0, 5.0, 5.0), Vec3::new(6.0, 7.0, 5.0));
        assert_eq!(model.object_box(1), Some(b));
        assert_eq!(model.object_box(2), None);
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
    fn self_evaluation_files() {
        // § 16: the kinds of file an evaluator brings.
        let read =
            |name: &str| std::fs::read_to_string(format!("../resources/test/{name}")).unwrap();
        let cube = parse_obj(&read("cube_vt_vn.obj")).unwrap();
        assert_eq!((cube.positions.len(), cube.triangles.len()), (8, 12)); // 6 quads × 2
        assert_eq!(cube.centre(), Vec3::new(3.0, 1.0, 1.0)); // not centred in the file
        let prism = parse_obj(&read("ngon_negative.obj")).unwrap();
        // 2 hexagons × 4 triangles + 6 quads × 2 = 20, from 8 faces.
        assert_eq!(prism.triangles.len(), 20);
        assert_eq!(prism.triangle_faces.iter().max(), Some(&7));
        // § 17: concave caps. L prism: 2 caps × 4 + 6 sides × 2 = 20;
        // star prism: 2 caps × 8 + 10 sides × 2 = 36. Sealed: no open edges.
        let concave = parse_obj(&read("concave.obj")).unwrap();
        assert_eq!(concave.triangles.len(), 20 + 36);
        assert_eq!(concave.open_edges(), 0);
        // § 18: two objects, one per prism.
        assert_eq!(concave.object_names, vec!["L_prism", "star_prism"]);
        let in_object = |o| concave.triangle_objects.iter().filter(|&&t| t == o).count();
        assert_eq!((in_object(0), in_object(1)), (20, 36));
        // `o Cube`, then `g sides`, `g caps`: Cube has no faces of its own → 2 objects.
        let cube = parse_obj(&read("cube_vt_vn.obj")).unwrap();
        assert_eq!(cube.object_names, vec!["sides", "caps"]);
        let err = parse_obj(&read("broken.obj")).unwrap_err();
        assert_eq!(err.line, 7);
        assert!(matches!(
            err.kind,
            ObjErrorKind::IndexOutOfRange {
                index: 7,
                defined: 3
            }
        ));
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
