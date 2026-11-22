//! From a parsed `.obj` to meshes ready to upload (§ 13, § 18, § 20 — moved into the
//! engine in § 24).
//!
//! - **One mesh per object** (`o` / `g` lines, § 18), so each can have its own picture.
//! - **Own corners per triangle** (§ 13): nothing is shared, so each face can have its
//!   own colour, UVs and normal.
//! - **Flat normals** (§ 20): every corner carries its face's normal.
//! - **UVs**: none, or **box mapping** (§ 18) fitted to each object's own box.
//! - **Colour from a callback** (§ 24): "face n → colour". The rule is the app's
//!   (scop's greys); the mechanism is the engine's.

use std::fmt;
use std::path::Path;

use crate::maths::Vec3;
use crate::mesh::{MeshData, Vertex};
use crate::obj::{ObjError, ObjModel, parse_obj};

/// How texture coordinates are made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UvMapping {
    /// All (0, 0): for models drawn without a picture.
    None,
    /// Box mapping (§ 18): each face uses the projection it faces most, one scale for
    /// u and v, fitted to its object's box. No streaks, no stretching.
    Box,
}

/// One mesh per object of `obj`. `colour(face)` gives each `.obj` face's colour
/// (linear RGB); faces count from 0 across the whole file.
pub fn build_meshes(
    obj: &ObjModel,
    uv: UvMapping,
    colour: impl Fn(u32) -> [f32; 3],
) -> Vec<MeshData> {
    let normals = face_normals(obj);
    (0..obj.object_names.len() as u32)
        .filter_map(|object| {
            // Each object's picture is fitted to its own box, not the whole file's.
            let (min, max) = obj.object_box(object)?;
            let mut vertices = Vec::new();
            let triangles = obj.triangles.iter().zip(&obj.triangle_faces);
            for ((triangle, &face), _) in triangles
                .zip(&obj.triangle_objects)
                .filter(|&(_, &o)| o == object)
            {
                let rgb = colour(face);
                let normal = normals[face as usize];
                let unit = normal.normalise(); // flat shading: one normal per face
                for &corner in triangle {
                    let p = obj.positions[corner as usize];
                    vertices.push(Vertex {
                        position: [p.x, p.y, p.z],
                        colour: rgb,
                        uv: match uv {
                            UvMapping::None => [0.0, 0.0],
                            UvMapping::Box => box_uv(p, normal, min, max),
                        },
                        normal: [unit.x, unit.y, unit.z],
                    });
                }
            }
            let indices = (0..vertices.len() as u32).collect();
            Some(MeshData { vertices, indices })
        })
        .collect()
}

/// Which way each `.obj` face looks (index = face number). A face's triangles each add
/// their cross product: together that's the face's Newell normal (§ 17), so a quad's two
/// triangles always agree — no seam through the middle of a face. Not normalised.
pub fn face_normals(obj: &ObjModel) -> Vec<Vec3> {
    let faces = obj
        .triangle_faces
        .iter()
        .max()
        .map_or(0, |&f| f as usize + 1);
    let mut normals = vec![Vec3::ZERO; faces];
    for (triangle, &face) in obj.triangles.iter().zip(&obj.triangle_faces) {
        let [a, b, c] = triangle.map(|i| obj.positions[i as usize]);
        normals[face as usize] = normals[face as usize] + (b - a).cross(c - a);
    }
    normals
}

/// The axis a direction points along most: 0 = x, 1 = y, 2 = z. Ties go to the
/// earlier axis, so the choice never flickers between two.
pub fn main_axis(n: Vec3) -> usize {
    let (x, y, z) = (n.x.abs(), n.y.abs(), n.z.abs());
    if x >= y && x >= z {
        0
    } else if y >= z {
        1
    } else {
        2
    }
}

/// Box mapping (§ 18): three projectors (front, side, top); a face uses the one it
/// faces most directly, so it's never edge-on to its projector — no streaks.
///
/// - u and v are both divided by the box's **largest** side: one scale, so the
///   picture's squares stay square (Q-18.2). The default fragment shader corrects for
///   the picture's own width ÷ height.
/// - Each projector is seen from the side the face looks out of: "right" on the picture
///   is the viewer's right, so a face looking the negative way flips u (otherwise it'd
///   be mirrored), and "up" stays +y on the side views.
pub fn box_uv(p: Vec3, normal: Vec3, min: Vec3, max: Vec3) -> [f32; 2] {
    let size = max - min;
    let scale = size.x.max(size.y).max(size.z);
    let scale = if scale > 0.0 { scale } else { 1.0 }; // a single point: any UV will do
    let from_min = (p - min) * (1.0 / scale); // 0 at min .. ≤ 1 at max
    let from_max = (max - p) * (1.0 / scale); // 0 at max .. ≤ 1 at min
    match main_axis(normal) {
        0 if normal.x >= 0.0 => [from_max.z, from_max.y], // seen from +x: right is −z
        0 => [from_min.z, from_max.y],                    // seen from −x: right is +z
        1 if normal.y >= 0.0 => [from_min.x, from_min.z], // top: right is +x
        1 => [from_max.x, from_min.z],                    // bottom: flipped
        _ if normal.z >= 0.0 => [from_min.x, from_max.y], // front: right is +x
        _ => [from_max.x, from_max.y],                    // back: right is −x
    }
}

/// A model read from a file and turned into meshes, not uploaded yet (§ 19).
pub struct LoadedObj {
    /// The file's name (not the whole path), e.g. for a window title.
    pub name: String,
    /// One mesh per object.
    pub meshes: Vec<MeshData>,
    /// Middle of the bounding box: what the model turns around.
    pub centre: Vec3,
    /// Half the bounding box's diagonal (at least 0.001, so the camera never sits on it).
    pub radius: f32,
    /// For reporting: corner and triangle counts, open edges (§ 17), object names.
    pub corners: usize,
    pub triangles: usize,
    pub open_edges: usize,
    pub object_names: Vec<String>,
}

/// Why a file couldn't be loaded as a model.
#[derive(Debug)]
pub enum LoadError {
    /// Couldn't read the file.
    Io(String, std::io::Error),
    /// Read it, but it isn't a valid `.obj`.
    Parse(String, ObjError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(path, err) => write!(f, "{path}: {err}"),
            LoadError::Parse(path, err) => write!(f, "{path}: {err}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// Reads and parses `path`, then builds its meshes (§ 19: the same loader for the
/// command line and dropped files). Nothing is uploaded: a broken file costs nothing.
pub fn load_obj(
    path: &Path,
    uv: UvMapping,
    colour: impl Fn(u32) -> [f32; 3],
) -> Result<LoadedObj, LoadError> {
    let shown = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|e| LoadError::Io(shown.clone(), e))?;
    let obj = parse_obj(&text).map_err(|e| LoadError::Parse(shown, e))?;
    let name = path.file_name().unwrap_or(path.as_os_str());
    Ok(LoadedObj {
        name: name.to_string_lossy().into_owned(),
        meshes: build_meshes(&obj, uv, colour),
        centre: obj.centre(),
        radius: obj.radius().max(0.001),
        corners: obj.positions.len(),
        triangles: obj.triangles.len(),
        open_edges: obj.open_edges(),
        object_names: obj.object_names.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: fn(u32) -> [f32; 3] = |_| [1.0; 3];

    fn read(name: &str) -> ObjModel {
        parse_obj(&std::fs::read_to_string(format!("../resources/{name}")).unwrap()).unwrap()
    }

    #[test]
    fn every_triangle_gets_its_own_corners_and_its_faces_colour() {
        let obj = parse_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
        let meshes = build_meshes(&obj, UvMapping::Box, |face| [face as f32 + 0.5; 3]);
        assert_eq!(meshes.len(), 1); // no `o` lines: one object
        let mesh = &meshes[0];
        // A quad → 2 triangles → 6 corners (not the 4 shared ones), indices 0..6.
        assert_eq!(mesh.vertices.len(), 6);
        assert_eq!(mesh.indices, vec![0, 1, 2, 3, 4, 5]);
        // The callback was asked about face 0 — every corner has its answer.
        assert!(mesh.vertices.iter().all(|v| v.colour == [0.5; 3]));
    }

    #[test]
    fn no_uv_mapping_means_zero_uvs() {
        let meshes = build_meshes(&read("42.obj"), UvMapping::None, WHITE);
        assert!(
            meshes
                .iter()
                .flat_map(|m| &m.vertices)
                .all(|v| v.uv == [0.0, 0.0])
        );
    }

    #[test]
    fn main_axis_is_the_largest_part_of_the_normal() {
        assert_eq!(main_axis(Vec3::new(0.9, 0.3, -0.2)), 0);
        assert_eq!(main_axis(Vec3::new(0.1, -0.8, 0.5)), 1);
        assert_eq!(main_axis(Vec3::new(0.0, 0.0, -1.0)), 2);
        assert_eq!(main_axis(Vec3::new(1.0, 1.0, 1.0)), 0); // ties → earlier axis
    }

    #[test]
    fn u_and_v_share_one_scale() {
        let (min, max) = (Vec3::ZERO, Vec3::new(4.0, 1.0, 0.0));
        let front = Vec3::new(0.0, 0.0, 1.0);
        assert_eq!(
            box_uv(Vec3::new(0.0, 1.0, 0.0), front, min, max),
            [0.0, 0.0]
        );
        assert_eq!(
            box_uv(Vec3::new(4.0, 0.0, 0.0), front, min, max),
            [1.0, 0.25]
        );
    }

    #[test]
    fn no_face_is_mirrored_and_up_stays_up() {
        let (min, max) = (Vec3::ZERO, Vec3::new(2.0, 2.0, 2.0));
        let middle = Vec3::new(1.0, 1.0, 1.0);
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let z = Vec3::new(0.0, 0.0, 1.0);
        // (face looks along, the viewer's right, the viewer's up)
        let views = [
            (x, -z, y),
            (-x, z, y),
            (z, x, y),
            (-z, -x, y),
            (y, x, -z),
            (-y, -x, -z),
        ];
        for (normal, right, up) in views {
            let here = box_uv(middle, normal, min, max);
            let to_right = box_uv(middle + right * 0.5, normal, min, max);
            let to_up = box_uv(middle + up * 0.5, normal, min, max);
            assert!(to_right[0] > here[0], "mirrored looking along {normal:?}");
            assert!(to_up[1] < here[1], "upside down looking along {normal:?}");
        }
    }

    /// Area of a triangle in 3D (half the cross product's length).
    fn area(a: Vec3, b: Vec3, c: Vec3) -> f32 {
        (b - a).cross(c - a).length() * 0.5
    }

    #[test]
    fn box_mapping_never_streaks() {
        // Each face keeps at least 1/√3 of its area in the picture (÷ the object's
        // scale²). § 14's single projector broke this on the logo's sides.
        for name in ["42.obj", "teapot.obj", "test/concave.obj"] {
            let obj = read(name);
            for (object, mesh) in build_meshes(&obj, UvMapping::Box, WHITE).iter().enumerate() {
                let (min, max) = obj.object_box(object as u32).unwrap();
                let size = max - min;
                let scale = size.x.max(size.y).max(size.z);
                for corners in mesh.vertices.chunks_exact(3) {
                    let p = |i: usize| {
                        let [x, y, z] = corners[i].position;
                        Vec3::new(x, y, z)
                    };
                    let uv = |i: usize| Vec3::new(corners[i].uv[0], corners[i].uv[1], 0.0);
                    let surface = area(p(0), p(1), p(2)) / (scale * scale);
                    let picture = area(uv(0), uv(1), uv(2));
                    assert!(
                        picture >= surface / 3f32.sqrt() * 0.99,
                        "{name}: a triangle streaks"
                    );
                }
            }
        }
    }

    #[test]
    fn each_object_gets_its_own_mesh_fitted_to_its_own_box() {
        let meshes = build_meshes(&read("test/concave.obj"), UvMapping::Box, WHITE);
        let corners: Vec<usize> = meshes.iter().map(|m| m.vertices.len()).collect();
        assert_eq!(corners, vec![60, 108]); // L prism 20 triangles, star prism 36
        for mesh in &meshes {
            let largest = mesh
                .vertices
                .iter()
                .flat_map(|v| v.uv)
                .fold(0.0_f32, f32::max);
            assert!((largest - 1.0).abs() < 1e-5, "largest UV {largest}");
        }
    }

    #[test]
    fn normals_are_unit_length_and_point_outwards() {
        let obj = read("test/cube_vt_vn.obj");
        let centre = obj.centre();
        for mesh in build_meshes(&obj, UvMapping::Box, WHITE) {
            for corners in mesh.vertices.chunks_exact(3) {
                let [nx, ny, nz] = corners[0].normal;
                let n = Vec3::new(nx, ny, nz);
                assert!((n.length() - 1.0).abs() < 1e-5);
                assert!(corners.iter().all(|c| c.normal == corners[0].normal)); // flat
                let [px, py, pz] = corners[0].position;
                assert!(
                    n.dot(Vec3::new(px, py, pz) - centre) > 0.0,
                    "inward normal {n:?}"
                );
            }
        }
    }

    #[test]
    fn a_broken_file_is_an_error_not_a_model() {
        let err = match load_obj(
            Path::new("../resources/test/broken.obj"),
            UvMapping::Box,
            WHITE,
        ) {
            Err(e) => e.to_string(),
            Ok(_) => panic!("a broken file loaded"),
        };
        assert!(
            err.contains("broken.obj") && err.contains("line 7"),
            "{err}"
        );
        assert!(load_obj(Path::new("../resources/missing.obj"), UvMapping::Box, WHITE).is_err());
        let logo = load_obj(Path::new("../resources/42.obj"), UvMapping::Box, WHITE).unwrap();
        assert_eq!(logo.name, "42.obj"); // the file name only
        assert_eq!(
            (logo.meshes.len(), logo.corners, logo.open_edges),
            (1, 42, 0)
        );
    }
}
