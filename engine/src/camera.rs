//! The orbit camera (§ 20, § 18, § 12 — moved into the engine in § 24).
//!
//! A camera on a sphere around the origin, always looking at it: mouse drags turn it
//! (yaw, pitch), the wheel zooms (field of view). It also works out the framing — how
//! far back to stand so a sphere of a given radius fits, and the near / far planes.
//!
//! Everything scop used to hard-code is a [`CameraSettings`] field, with scop's
//! values as the defaults (§ 24: settings, not assumptions) — e.g. the up axis.

use crate::maths::{Mat4, Vec3};

/// Which way is up in the app's world (§ 24, Q-24.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Up {
    /// +y up (scop, and most model files). The camera starts on +z.
    Y,
    /// +z up (many physics and CAD worlds). The camera starts on −y.
    Z,
}

impl Up {
    /// (side, up, start): "right" at the start, "up", and where the camera starts
    /// (unit directions). Right-handed in both cases.
    fn basis(self) -> (Vec3, Vec3, Vec3) {
        match self {
            Up::Y => (
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ),
            Up::Z => (
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(0.0, -1.0, 0.0),
            ),
        }
    }

    /// The "up" direction itself.
    pub fn vector(self) -> Vec3 {
        self.basis().1
    }
}

/// Everything about the camera an app may want different. `Default` = scop's values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraSettings {
    /// Which way is up.
    pub up: Up,
    /// The pitch clamp (Q-20.2): how far over / under the camera may go, in radians.
    /// Just short of straight up / down, so it never flips.
    pub pitch_limit: f32,
    /// How far one pixel of mouse drag turns the camera, in radians.
    pub radians_per_pixel: f32,
    /// Vertical field of view at the start and after `reset`, in degrees. Framing
    /// always uses this one, so zooming doesn't move the camera (§ 18).
    pub start_fov_degrees: f32,
    /// Zoom limits, in degrees: near 0° the image blows up, near 180° it turns inside out.
    pub min_fov_degrees: f32,
    pub max_fov_degrees: f32,
    /// Each wheel notch multiplies the field of view by this (0.9 = 10 % narrower).
    pub zoom_per_notch: f32,
    /// Near plane as a share of the model's radius: as large as possible, for depth
    /// precision (§ 11).
    pub near_share: f32,
    /// How many radii of room there are behind the model before the far plane.
    pub far_margin: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        CameraSettings {
            up: Up::Y,
            pitch_limit: 89.0_f32.to_radians(),
            radians_per_pixel: 0.005,
            start_fov_degrees: 60.0,
            min_fov_degrees: 10.0,
            max_fov_degrees: 100.0,
            zoom_per_notch: 0.9,
            near_share: 0.01,
            far_margin: 20.0,
        }
    }
}

/// Where the camera stands and how far apart the near and far planes are, for a model
/// that fits in a sphere of some radius (§ 12).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Framing {
    /// From the origin (the model's centre) to the camera.
    pub distance: f32,
    pub near: f32,
    pub far: f32,
}

/// The orbit camera's state: where it is on its sphere, and the zoom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    /// Round the model (left / right), in radians. 0 = at the start position.
    pub yaw: f32,
    /// Over / under the model, in radians, kept within ±`pitch_limit`.
    pub pitch: f32,
    /// Vertical field of view in degrees: the zoom.
    pub fov_degrees: f32,
    pub settings: CameraSettings,
}

impl OrbitCamera {
    /// At the start position, unzoomed.
    pub fn new(settings: CameraSettings) -> Self {
        OrbitCamera {
            yaw: 0.0,
            pitch: 0.0,
            fov_degrees: settings.start_fov_degrees,
            settings,
        }
    }

    /// Back to the start position and zoom (settings stay).
    pub fn reset(&mut self) {
        *self = OrbitCamera::new(self.settings);
    }

    /// A mouse drag of (dx, dy) pixels. Like grabbing the model: drag right and it
    /// turns right (the camera walks left), drag down and the camera climbs.
    pub fn drag(&mut self, dx: f32, dy: f32) {
        let step = self.settings.radians_per_pixel;
        let limit = self.settings.pitch_limit;
        self.yaw -= dx * step;
        self.pitch = (self.pitch + dy * step).clamp(-limit, limit);
    }

    /// The wheel moved `notches` (+ = zoom in). Multiplying, not adding, so every
    /// notch feels the same at any zoom level; clamped (§ 18).
    pub fn zoom(&mut self, notches: f32) {
        let s = &self.settings;
        self.fov_degrees = (self.fov_degrees * s.zoom_per_notch.powf(notches))
            .clamp(s.min_fov_degrees, s.max_fov_degrees);
    }

    /// Framing for a model of `radius` around the origin (§ 12). The distance always
    /// comes from the *starting* field of view: from the zoomed one, the camera would
    /// back off exactly as much as the zoom magnifies — no zoom at all (§ 18).
    pub fn framing(&self, radius: f32) -> Framing {
        let s = &self.settings;
        let distance = radius / (s.start_fov_degrees.to_radians() / 2.0).sin();
        Framing {
            distance,
            near: radius * s.near_share,
            far: distance + radius * s.far_margin,
        }
    }

    /// Where the camera stands, `distance` from the origin.
    pub fn eye(&self, distance: f32) -> Vec3 {
        let (side, up, start) = self.settings.up.basis();
        let level = distance * self.pitch.cos(); // how far out, seen from above
        side * (level * self.yaw.sin())
            + start * (level * self.yaw.cos())
            + up * (distance * self.pitch.sin())
    }

    /// The view card: standing at `eye(distance)`, looking at the origin (§ 20).
    pub fn view(&self, distance: f32) -> Mat4 {
        Mat4::look_at(self.eye(distance), Vec3::ZERO, self.settings.up.vector())
    }

    /// The projection card with the current zoom (§ 9, § 18).
    pub fn projection(&self, aspect: f32, framing: Framing) -> Mat4 {
        Mat4::perspective(
            self.fov_degrees.to_radians(),
            aspect,
            framing.near,
            framing.far,
        )
    }
}

/// The nearest and farthest a model of `radius` can be from the camera, when its
/// centre is `centre_distance` away (§ 19 depth mode). Turning doesn't matter: the
/// model always fits in the sphere.
pub fn depth_range(centre_distance: f32, radius: f32) -> (f32, f32) {
    (
        (centre_distance - radius).max(0.0),
        centre_distance + radius,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maths::Vec4;

    #[test]
    fn starts_in_front_and_never_goes_over_the_top() {
        let mut camera = OrbitCamera::new(CameraSettings::default());
        // Start: straight in front, on +z — the pre-§ 20 camera.
        let eye = camera.eye(5.0);
        assert!((eye.z - 5.0).abs() < 1e-5 && eye.x.abs() < 1e-5 && eye.y.abs() < 1e-5);
        // Drag a long way down (camera climbs): stops at the clamp, never past it.
        let limit = camera.settings.pitch_limit;
        camera.drag(0.0, 100_000.0);
        assert_eq!(camera.pitch, limit);
        camera.drag(0.0, -200_000.0);
        assert_eq!(camera.pitch, -limit);
        // Wherever it stands, the camera stays at the framing distance.
        camera.drag(1234.0, 3000.0);
        assert!((camera.eye(5.0).length() - 5.0).abs() < 1e-4);
    }

    #[test]
    fn drag_right_walks_the_camera_left() {
        let mut camera = OrbitCamera::new(CameraSettings::default());
        camera.drag(100.0, 0.0);
        assert!(camera.eye(5.0).x < 0.0);
    }

    #[test]
    fn z_up_worlds_start_on_minus_y_and_climb_along_z() {
        // § 24, Q-24.5: the up axis is a setting.
        let settings = CameraSettings {
            up: Up::Z,
            ..CameraSettings::default()
        };
        let mut camera = OrbitCamera::new(settings);
        let start = camera.eye(5.0);
        assert!((start.y + 5.0).abs() < 1e-5 && start.z.abs() < 1e-5);
        camera.drag(0.0, 100.0); // climb
        assert!(camera.eye(5.0).z > 0.0);
        // A point above the origin (along +z) is above it on screen too.
        let above = (camera.view(5.0) * Vec4::point(Vec3::new(0.0, 0.0, 1.0))).xyz();
        assert!(above.y > 0.0, "{above:?}");
    }

    #[test]
    fn zoom_multiplies_and_clamps() {
        let mut camera = OrbitCamera::new(CameraSettings::default());
        camera.zoom(1.0);
        assert!((camera.fov_degrees - 54.0).abs() < 1e-4); // one notch: 10 % narrower
        camera.zoom(-1.0);
        assert!((camera.fov_degrees - 60.0).abs() < 1e-4); // and back
        camera.zoom(100.0);
        assert_eq!(camera.fov_degrees, camera.settings.min_fov_degrees);
        camera.zoom(-200.0);
        assert_eq!(camera.fov_degrees, camera.settings.max_fov_degrees);
        camera.reset();
        assert_eq!(camera.fov_degrees, 60.0);
    }

    #[test]
    fn framing_ignores_the_zoom() {
        let mut camera = OrbitCamera::new(CameraSettings::default());
        let before = camera.framing(1.0);
        assert!((before.distance - 2.0).abs() < 1e-5); // 1 / sin 30°
        assert!((before.near - 0.01).abs() < 1e-6 && (before.far - 22.0).abs() < 1e-5);
        camera.zoom(5.0);
        assert_eq!(camera.framing(1.0), before); // zooming never moves the camera
    }

    #[test]
    fn depth_range_follows_the_model() {
        assert_eq!(depth_range(10.0, 2.0), (8.0, 12.0));
        assert_eq!(depth_range(7.0, 2.0), (5.0, 9.0));
        assert_eq!(depth_range(1.0, 2.0).0, 0.0); // never a negative distance
    }
}
