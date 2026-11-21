//! The sky (§ 23): a cube map scop makes itself at start — or six `.tga` faces from
//! `resources/sky/` if they're all there.
//!
//! Generating it works backwards from the lookup: for every texel of every face, find
//! the direction the GPU would use to reach it, and colour it by that direction. The
//! sky then looks right whatever the face conventions, as long as `texel_direction`
//! matches the GPU's table (`face_for` below is that table, for the tests).

use engine::{Image, Vec3};

/// Texels per face side. 256² × 6 faces ≈ 1.5 MB: smooth enough for a gradient.
pub const FACE_SIZE: u32 = 256;
/// Face file names in Vulkan's layer order: +x, −x, +y, −y, +z, −z.
pub const FACE_NAMES: [&str; 6] = ["px", "nx", "py", "ny", "pz", "nz"];

/// Colours, linear RGB (the texture is sRGB: they're encoded when stored).
const ZENITH: [f32; 3] = [0.10, 0.25, 0.65];
const HORIZON: [f32; 3] = [0.70, 0.78, 0.88];
const GROUND_NEAR: [f32; 3] = [0.30, 0.28, 0.25];
const GROUND_FAR: [f32; 3] = [0.06, 0.055, 0.05];
const SUN: [f32; 3] = [1.0, 0.95, 0.80];

/// The direction the GPU uses to reach texel (s, t) of `face` — s across, t down,
/// both 0..1 — following Vulkan's cube-map table. Not unit length (it doesn't matter
/// for a lookup).
pub fn texel_direction(face: usize, s: f32, t: f32) -> Vec3 {
    let (a, b) = (2.0 * s - 1.0, 2.0 * t - 1.0);
    match face {
        0 => Vec3::new(1.0, -b, -a),  // +x
        1 => Vec3::new(-1.0, -b, a),  // −x
        2 => Vec3::new(a, 1.0, b),    // +y
        3 => Vec3::new(a, -1.0, -b),  // −y
        4 => Vec3::new(a, -b, 1.0),   // +z
        _ => Vec3::new(-a, -b, -1.0), // −z
    }
}

/// The sky's colour looking in `direction` (unit length), with the sun towards `sun`
/// (unit length). Linear RGB.
pub fn sky_colour(direction: Vec3, sun: Vec3) -> [f32; 3] {
    let up = direction.y;
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| -> [f32; 3] {
        [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
    };
    let base = if up >= 0.0 {
        mix(HORIZON, ZENITH, up.sqrt()) // fades quickly from the horizon upwards
    } else {
        mix(GROUND_NEAR, GROUND_FAR, (-up).sqrt())
    };
    // The sun: a bright disc, and a soft glow around it.
    let towards = direction.dot(sun).max(0.0);
    let glow = towards.powi(64) * 0.5;
    let disc = if towards > 0.9995 { 1.0 } else { 0.0 };
    [0, 1, 2].map(|i| (base[i] + SUN[i] * glow).max(SUN[i] * disc).min(1.0))
}

/// The six faces of the generated sky, the sun towards `sun`.
pub fn generate(sun: Vec3) -> [Image; 6] {
    let sun = sun.normalise();
    std::array::from_fn(|face| {
        let mut rgba = Vec::with_capacity((FACE_SIZE * FACE_SIZE * 4) as usize);
        for y in 0..FACE_SIZE {
            for x in 0..FACE_SIZE {
                // The middle of the texel.
                let s = (x as f32 + 0.5) / FACE_SIZE as f32;
                let t = (y as f32 + 0.5) / FACE_SIZE as f32;
                let colour = sky_colour(texel_direction(face, s, t).normalise(), sun);
                rgba.extend(colour.map(srgb_byte));
                rgba.push(255);
            }
        }
        Image {
            width: FACE_SIZE,
            height: FACE_SIZE,
            rgba,
        }
    })
}

/// Linear 0..1 → an sRGB-encoded byte, as the texture format expects (§ 14). The
/// standard sRGB curve; given, not derived.
fn srgb_byte(linear: f32) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    let encoded = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vulkan's lookup table, the other way round: which face a direction lands on,
    /// and where (s, t). From the spec's cube-map face selection table.
    fn face_for(d: Vec3) -> (usize, f32, f32) {
        let (ax, ay, az) = (d.x.abs(), d.y.abs(), d.z.abs());
        let (face, sc, tc, ma) = if ax >= ay && ax >= az {
            if d.x > 0.0 {
                (0, -d.z, -d.y, ax)
            } else {
                (1, d.z, -d.y, ax)
            }
        } else if ay >= az {
            if d.y > 0.0 {
                (2, d.x, d.z, ay)
            } else {
                (3, d.x, -d.z, ay)
            }
        } else if d.z > 0.0 {
            (4, d.x, -d.y, az)
        } else {
            (5, -d.x, -d.y, az)
        };
        (face, (sc / ma + 1.0) / 2.0, (tc / ma + 1.0) / 2.0)
    }

    #[test]
    fn every_texel_is_reached_by_its_own_direction() {
        // Generate by direction, look up by direction: the round trip must land on the
        // same face and spot, or the sky would come out in the wrong place.
        for face in 0..6 {
            for &(s, t) in &[(0.1, 0.2), (0.5, 0.5), (0.9, 0.3), (0.25, 0.8)] {
                let (f, s2, t2) = face_for(texel_direction(face, s, t));
                assert_eq!(f, face);
                assert!(
                    (s - s2).abs() < 1e-5 && (t - t2).abs() < 1e-5,
                    "face {face}"
                );
            }
        }
    }

    #[test]
    fn pale_horizon_blue_overhead_dark_ground_sun_brightest() {
        let sun = Vec3::new(-1.0, 1.0, 1.0).normalise();
        let brightness = |d: Vec3| sky_colour(d.normalise(), sun).iter().sum::<f32>();
        let up = brightness(Vec3::new(0.0, 1.0, 0.0));
        let horizon = brightness(Vec3::new(1.0, 0.0, 0.0));
        let down = brightness(Vec3::new(0.0, -1.0, 0.0));
        assert!(horizon > up && up > down, "sky gradient");
        // The sun outshines any other direction.
        let at_sun = brightness(sun);
        for d in [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 0.2, 0.0),
            Vec3::new(0.0, 0.3, -1.0),
        ] {
            assert!(at_sun > brightness(d));
        }
    }

    #[test]
    fn faces_are_square_and_srgb_encoded() {
        let faces = generate(Vec3::new(0.0, 1.0, 0.0));
        for face in &faces {
            assert_eq!((face.width, face.height), (FACE_SIZE, FACE_SIZE));
            assert_eq!(face.rgba.len(), (FACE_SIZE * FACE_SIZE * 4) as usize);
        }
        assert_eq!(srgb_byte(0.0), 0);
        assert_eq!(srgb_byte(1.0), 255);
        assert_eq!(srgb_byte(0.215_861), 128); // linear ≈ 0.216 is sRGB mid-grey (128)
    }
}
