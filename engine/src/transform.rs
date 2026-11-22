//! Where a model is and how it's turned (§ 13 — moved into the engine in § 24).

use crate::maths::{Mat4, Vec3};

/// A model's place and turn: the numbers keys change, turned into a model card.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    /// Turns around x, y, z, in radians (Euler angles).
    pub angles: Vec3,
}

impl Transform {
    /// Not moved, not turned.
    pub const START: Transform = Transform {
        position: Vec3::ZERO,
        angles: Vec3::ZERO,
    };

    /// The § 9 recipe, read right to left: move `pivot` to the origin, roll (z), tilt
    /// (x), spin (y), then move. Moving is leftmost, so movement always follows the
    /// screen's axes, whatever way the model is turned (Q-13.4).
    pub fn matrix(&self, pivot: Vec3) -> Mat4 {
        Mat4::translation(self.position)
            * Mat4::rotation_y(self.angles.y)
            * Mat4::rotation_x(self.angles.x)
            * Mat4::rotation_z(self.angles.z)
            * Mat4::translation(-pivot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maths::Vec4;

    #[test]
    fn translation_is_applied_after_the_turn() {
        // Q-13.4: turned 90° around y, then moved right → still right on screen.
        let mut t = Transform::START;
        t.angles.y = std::f32::consts::FRAC_PI_2;
        t.position = Vec3::new(1.0, 0.0, 0.0);
        let moved_to = (t.matrix(Vec3::ZERO) * Vec4::point(Vec3::ZERO)).xyz();
        assert!((moved_to - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn the_pivot_lands_on_the_position() {
        let t = Transform {
            position: Vec3::new(0.0, 2.0, 0.0),
            angles: Vec3::new(0.3, 1.1, -0.4),
        };
        let pivot = Vec3::new(5.0, -1.0, 3.0);
        let landed = (t.matrix(pivot) * Vec4::point(pivot)).xyz();
        assert!((landed - t.position).length() < 1e-5);
    }
}
