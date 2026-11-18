//! Splitting a face into triangles (§ 17): ear clipping on the face's flattened outline.
//!
//! 1. Newell's normal: which way the face looks, using *every* corner, so an inward
//!    corner can't flip it (a single cross product could).
//! 2. Flatten: drop the coordinate the normal points along most → a 2D outline.
//! 3. Ear clipping: repeatedly cut off a corner whose triangle lies fully inside the
//!    outline (it turns outwards, and no other corner is inside it). Every cut piece
//!    is kept; the last 3 corners are the last piece: `n − 2` triangles in all.
//! 4. If no corner is an ear (a broken, self-crossing face), fan the rest: the loop
//!    always ends and the face is still drawn.

use crate::maths::Vec3;

/// Below this, a turn counts as straight (collinear corners are never ears).
const FLAT: f32 = 1e-12;

/// Picks the two coordinates kept when flattening (e.g. `(y, z)` when dropping x).
type Pick = fn(&Vec3) -> (f32, f32);

/// Triangles covering the face whose corners are `corners` (indices into `positions`),
/// in the face's own corner order. Faces with 3 corners come back unchanged.
pub(crate) fn triangulate(positions: &[Vec3], corners: &[u32]) -> Vec<[u32; 3]> {
    if corners.len() <= 3 {
        return fan(corners);
    }
    let points: Vec<Vec3> = corners.iter().map(|&c| positions[c as usize]).collect();
    let Some(outline) = flatten(&points) else {
        return fan(corners); // no area at all (all corners on a line): nothing to clip
    };

    // `remaining` holds positions in `corners` (0..n), in outline order.
    let mut remaining: Vec<usize> = (0..corners.len()).collect();
    let mut triangles = Vec::with_capacity(corners.len() - 2);
    while remaining.len() > 3 {
        let Some(i) = find_ear(&outline, &remaining) else {
            // Broken face (e.g. a bow-tie): no ear exists. Fan what's left (§ 17, Q-17.4).
            let rest: Vec<u32> = remaining.iter().map(|&k| corners[k]).collect();
            triangles.extend(fan(&rest));
            return triangles;
        };
        let n = remaining.len();
        let (prev, cur, next) = (
            remaining[(i + n - 1) % n],
            remaining[i],
            remaining[(i + 1) % n],
        );
        triangles.push([corners[prev], corners[cur], corners[next]]); // the snipped piece
        remaining.remove(i);
    }
    // The last 3 corners are already a triangle: the final piece.
    triangles.push([
        corners[remaining[0]],
        corners[remaining[1]],
        corners[remaining[2]],
    ]);
    triangles
}

/// The § 12 fan: (c0, c1, c2), (c0, c2, c3), … Correct for convex faces.
fn fan(corners: &[u32]) -> Vec<[u32; 3]> {
    corners[1..]
        .windows(2)
        .map(|pair| [corners[0], pair[0], pair[1]])
        .collect()
}

/// Newell's normal, then the 2D outline seen from that side, turned so that the
/// outline always goes **anticlockwise** (outward corners turn left, cross > 0).
/// `None` if the face has no area.
fn flatten(points: &[Vec3]) -> Option<Vec<(f32, f32)>> {
    // Newell: three running sums over every edge a → b.
    let mut n = Vec3::ZERO;
    for (k, a) in points.iter().enumerate() {
        let b = points[(k + 1) % points.len()];
        n.x += (a.y - b.y) * (a.z + b.z);
        n.y += (a.z - b.z) * (a.x + b.x);
        n.z += (a.x - b.x) * (a.y + b.y);
    }
    let (ax, ay, az) = (n.x.abs(), n.y.abs(), n.z.abs());
    if ax.max(ay).max(az) <= FLAT {
        return None;
    }
    // Drop the axis the normal points along most. The pairs (y,z), (z,x), (x,y) keep
    // the right-handed order, so the outline is anticlockwise when that component > 0.
    let (pick, facing): (Pick, f32) = if ax >= ay && ax >= az {
        (|p| (p.y, p.z), n.x)
    } else if ay >= az {
        (|p| (p.z, p.x), n.y)
    } else {
        (|p| (p.x, p.y), n.z)
    };
    let mut outline: Vec<(f32, f32)> = points.iter().map(pick).collect();
    if facing < 0.0 {
        // Seen from behind: mirror it so "turns left" still means "points outwards".
        for p in &mut outline {
            p.0 = -p.0;
        }
    }
    Some(outline)
}

/// The first corner (position in `remaining`) that's an ear, if any.
///
/// The search starts at corner 1, so on a convex face (where every corner is an ear)
/// it cuts (c0, c1, c2), then (c0, c2, c3), … — exactly § 12's fan. Faces that were
/// already right stay byte-for-byte the same; only concave ones change.
fn find_ear(outline: &[(f32, f32)], remaining: &[usize]) -> Option<usize> {
    let n = remaining.len();
    (1..n).chain(0..1).find(|&i| {
        let a = outline[remaining[(i + n - 1) % n]];
        let b = outline[remaining[i]];
        let c = outline[remaining[(i + 1) % n]];
        // Test 1: the corner turns outwards (left, on an anticlockwise outline).
        if turn(a, b, c) <= FLAT {
            return false;
        }
        // Test 2: no other remaining corner is inside the triangle.
        remaining
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != i && j != (i + n - 1) % n && j != (i + 1) % n)
            .all(|(_, &k)| !inside(outline[k], a, b, c))
    })
}

/// Which way the path a → b → c turns: > 0 left, < 0 right, 0 straight (2D cross product).
fn turn(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0)
}

/// Is `p` inside (or on the edge of) the anticlockwise triangle a, b, c?
/// Inside = on the left of (or on) all three edges.
fn inside(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    turn(a, b, p) >= 0.0 && turn(b, c, p) >= 0.0 && turn(c, a, p) >= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Corners on the z = 0 plane from (x, y) pairs.
    fn flat(points: &[(f32, f32)]) -> Vec<Vec3> {
        points.iter().map(|&(x, y)| Vec3::new(x, y, 0.0)).collect()
    }

    fn area(positions: &[Vec3], triangles: &[[u32; 3]]) -> f32 {
        triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| positions[i as usize]);
                (b - a).cross(c - a).length() / 2.0
            })
            .sum()
    }

    fn all(n: u32) -> Vec<u32> {
        (0..n).collect()
    }

    /// The L from the § 17 walkthrough: A B C D E F, area 3 (three unit squares).
    const L: [(f32, f32); 6] = [
        (0.0, 2.0),
        (1.0, 2.0),
        (1.0, 1.0),
        (2.0, 1.0),
        (2.0, 0.0),
        (0.0, 0.0),
    ];

    #[test]
    fn l_shape_area_check() {
        let pos = flat(&L);
        let ears = triangulate(&pos, &all(6));
        assert_eq!(ears.len(), 4); // n − 2
        assert!((area(&pos, &ears) - 3.0).abs() < 1e-5); // covers exactly the L
        // The fan from A includes A-C-D across the notch: more than the L (Q-17.2).
        assert!(area(&pos, &fan(&all(6))) > 3.0 + 1e-3);
    }

    #[test]
    fn works_whichever_way_round_and_facing() {
        // Same L listed clockwise, and lying in a tilted plane facing −x.
        let reversed: Vec<(f32, f32)> = L.iter().rev().copied().collect();
        let pos = flat(&reversed);
        assert!((area(&pos, &triangulate(&pos, &all(6))) - 3.0).abs() < 1e-5);
        let tilted: Vec<Vec3> = L.iter().map(|&(a, b)| Vec3::new(0.0, b, a)).collect();
        assert!((area(&tilted, &triangulate(&tilted, &all(6))) - 3.0).abs() < 1e-5);
    }

    #[test]
    fn star_area_check() {
        // A 5-point star: 10 corners, alternating outer (r = 2) and inner (r = 0.8).
        let star: Vec<(f32, f32)> = (0..10)
            .map(|k| {
                let r = if k % 2 == 0 { 2.0 } else { 0.8 };
                let a = std::f32::consts::PI / 5.0 * k as f32;
                (r * a.cos(), r * a.sin())
            })
            .collect();
        let pos = flat(&star);
        let ears = triangulate(&pos, &all(10));
        assert_eq!(ears.len(), 8);
        // Shoelace formula: the outline's own area.
        let shoelace: f32 = (0..10)
            .map(|k| {
                let (a, b) = (star[k], star[(k + 1) % 10]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum::<f32>()
            / 2.0;
        assert!((area(&pos, &ears) - shoelace.abs()).abs() < 1e-4);
    }

    #[test]
    fn convex_faces_match_the_old_fan() {
        // A convex hexagon: ear clipping gives exactly § 12's fan.
        let hex: Vec<(f32, f32)> = (0..6)
            .map(|k| {
                let a = std::f32::consts::PI / 3.0 * k as f32;
                (a.cos(), a.sin())
            })
            .collect();
        let pos = flat(&hex);
        assert_eq!(triangulate(&pos, &all(6)), fan(&all(6)));
    }

    #[test]
    fn convex_faces_and_triangles_still_work() {
        let square = flat(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
        let t = triangulate(&square, &all(4));
        assert_eq!(t.len(), 2);
        assert!((area(&square, &t) - 1.0).abs() < 1e-6);
        assert_eq!(triangulate(&square, &[0, 1, 2]), vec![[0, 1, 2]]);
    }

    #[test]
    fn bent_quad_gives_two_triangles() {
        let pos = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.5), // lifted: not flat
            Vec3::new(0.0, 1.0, 0.0),
        ];
        assert_eq!(triangulate(&pos, &all(4)).len(), 2);
    }

    #[test]
    fn broken_faces_never_hang() {
        // Bow-tie: edges cross.
        let bowtie = flat(&[(0.0, 0.0), (1.0, 1.0), (1.0, 0.0), (0.0, 1.0)]);
        assert_eq!(triangulate(&bowtie, &all(4)).len(), 2);
        // All on one line: no area at all.
        let line = flat(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]);
        assert_eq!(triangulate(&line, &all(4)).len(), 2);
    }
}
