//! Just enough 3D maths (§ 9): `Vec3`, `Vec4` and `Mat4`, written by hand
//! (the subject forbids a maths library).
//!
//! Convention — every maths bug report starts here:
//! 1. Right-handed: x right, y up, z towards you; the camera looks down −z.
//! 2. Column vectors: `m * v`. Chains read RIGHT TO LEFT: `a * b * v` = b first, then a.
//! 3. Column-major storage, like GLSL's `mat4`: `cols[c][r]`.
//! 4. Vulkan's screen (y down, depth 0..1) is handled by `perspective` only.

use std::ops::{Add, Mul, Neg, Sub};

/// Below this, two floats count as equal (and a vector counts as zero length).
pub const EPSILON: f32 = 1e-5;

// ---------------------------------------------------------------------------
// Vec3 — three numbers: a place or a direction (which one is decided by `w`
// only when it becomes a `Vec4`).
// ---------------------------------------------------------------------------

/// x, y, z. Plain numbers, owns nothing, so `Copy`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    /// One number: 1 = same way, 0 = at right angles, −1 = opposite
    /// (for length-1 vectors).
    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    /// A new vector at right angles to both (right-handed: x × y = z).
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    /// How long the arrow is (Pythagoras in 3D).
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// Same direction, length 1. A zero-length vector has no direction:
    /// it is returned as-is instead of dividing by zero (which gives NaN).
    pub fn normalise(self) -> Vec3 {
        let len = self.length();
        if len < EPSILON {
            return self;
        }
        self * (1.0 / len)
    }
}

// Operator overloading: lets us write `a + b` instead of `a.add(b)`.
impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

/// Vector × number: resize the arrow.
impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, k: f32) -> Vec3 {
        Vec3::new(self.x * k, self.y * k, self.z * k)
    }
}

// ---------------------------------------------------------------------------
// Vec4 — a Vec3 plus `w`: 1 = point (moves), 0 = direction (doesn't).
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Vec4 { x, y, z, w }
    }

    /// A place: `w = 1`, so translations apply.
    pub const fn point(v: Vec3) -> Self {
        Vec4::new(v.x, v.y, v.z, 1.0)
    }

    /// A direction: `w = 0`, so translations cancel out.
    pub const fn direction(v: Vec3) -> Self {
        Vec4::new(v.x, v.y, v.z, 0.0)
    }

    /// Drop `w` (no divide).
    pub const fn xyz(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    fn as_array(self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }
}

// ---------------------------------------------------------------------------
// Mat4 — a "recipe card". Each row computes one output number.
// ---------------------------------------------------------------------------

/// 4×4 matrix, stored column by column (`cols[c][r]`) because GLSL reads it
/// that way. `repr(C)`: 16 floats in a row, no surprises, ready for § 10.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    cols: [[f32; 4]; 4],
}

impl Mat4 {
    /// Write a matrix the way it looks on paper (rows), store it as columns.
    /// Every card below uses this, so the code matches the pictures in § 9.
    pub(crate) const fn from_rows(r: [[f32; 4]; 4]) -> Self {
        Mat4 {
            cols: [
                [r[0][0], r[1][0], r[2][0], r[3][0]],
                [r[0][1], r[1][1], r[2][1], r[3][1]],
                [r[0][2], r[1][2], r[2][2], r[3][2]],
                [r[0][3], r[1][3], r[2][3], r[3][3]],
            ],
        }
    }

    /// Entry at (row, col), as on paper.
    fn at(&self, row: usize, col: usize) -> f32 {
        self.cols[col][row]
    }

    /// The 16 numbers in memory order: column by column, like GLSL's `mat4`
    /// (and std140, § 10). This is what gets copied to the GPU.
    pub const fn to_cols_array(self) -> [[f32; 4]; 4] {
        self.cols
    }

    /// The "do nothing" card: every number copied straight across.
    pub const IDENTITY: Mat4 = Mat4::from_rows([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    /// Move card: the move sits in the 4th column, and gets multiplied by `w`.
    pub const fn translation(t: Vec3) -> Self {
        Mat4::from_rows([
            [1.0, 0.0, 0.0, t.x],
            [0.0, 1.0, 0.0, t.y],
            [0.0, 0.0, 1.0, t.z],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Resize card: how much to stretch each axis, on the diagonal.
    pub const fn scale(s: Vec3) -> Self {
        Mat4::from_rows([
            [s.x, 0.0, 0.0, 0.0],
            [0.0, s.y, 0.0, 0.0],
            [0.0, 0.0, s.z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Turn card around the x axis, through the origin. Angle in radians.
    pub fn rotation_x(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Mat4::from_rows([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, c, -s, 0.0],
            [0.0, s, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Turn card around the y axis (the vertical one), through the origin.
    pub fn rotation_y(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Mat4::from_rows([
            [c, 0.0, s, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [-s, 0.0, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Turn card around the z axis, through the origin.
    pub fn rotation_z(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Mat4::from_rows([
            [c, -s, 0.0, 0.0],
            [s, c, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// View card (§ 20): a camera standing at `eye`, looking at `target`, with `up`
    /// roughly its up. The result puts the camera at the origin looking down −z,
    /// like every view so far. Given, not derived (owner's choice); the tests show
    /// what it does. `up` must not point along the view direction (that's the flip
    /// the pitch clamp prevents, Q-20.2).
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let forward = (target - eye).normalise();
        let right = forward.cross(up).normalise();
        let true_up = right.cross(forward);
        Mat4::from_rows([
            [right.x, right.y, right.z, -right.dot(eye)],
            [true_up.x, true_up.y, true_up.z, -true_up.dot(eye)],
            [-forward.x, -forward.y, -forward.z, forward.dot(eye)],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Perspective card, Vulkan flavour. `fov_y` in radians, `aspect` =
    /// width ÷ height, `near`/`far` = positive distances in front of the camera.
    ///
    /// Remember, don't derive: last row copies the distance into `w` (the GPU
    /// then divides by it: far = small); third row squeezes near..far into
    /// depth 0..1; `-f` flips y for Vulkan; `/aspect` undoes the window stretch.
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y / 2.0).tan();
        Mat4::from_rows([
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, -f, 0.0, 0.0],
            [0.0, 0.0, far / (near - far), near * far / (near - far)],
            [0.0, 0.0, -1.0, 0.0],
        ])
    }
}

/// Card × card = one combined card. `a * b` does b first, then a.
/// Entry (r, c) = row r of `self` · column c of `rhs`.
impl Mul for Mat4 {
    type Output = Mat4;
    fn mul(self, rhs: Mat4) -> Mat4 {
        let mut out = [[0.0; 4]; 4];
        for (c, out_col) in out.iter_mut().enumerate() {
            for (r, cell) in out_col.iter_mut().enumerate() {
                *cell = (0..4).map(|k| self.at(r, k) * rhs.at(k, c)).sum();
            }
        }
        Mat4 { cols: out }
    }
}

/// Card × vector = apply the recipe. Output number r = row r · (x, y, z, w).
impl Mul<Vec4> for Mat4 {
    type Output = Vec4;
    fn mul(self, v: Vec4) -> Vec4 {
        let v = v.as_array();
        let row = |r: usize| (0..4).map(|k| self.at(r, k) * v[k]).sum();
        Vec4::new(row(0), row(1), row(2), row(3))
    }
}

// ---------------------------------------------------------------------------
// Tests — every maths bug shows up as a model in the wrong place, so each
// card is checked on a point whose answer we know.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    /// Floats are never exactly equal after maths: compare with a tolerance.
    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn assert_vec3(a: Vec3, b: Vec3) {
        assert!(
            close(a.x, b.x) && close(a.y, b.y) && close(a.z, b.z),
            "{a:?} != {b:?}"
        );
    }

    fn assert_vec4(a: Vec4, b: Vec4) {
        assert!(
            close(a.x, b.x) && close(a.y, b.y) && close(a.z, b.z) && close(a.w, b.w),
            "{a:?} != {b:?}"
        );
    }

    fn assert_mat(a: Mat4, b: Mat4) {
        for c in 0..4 {
            for r in 0..4 {
                assert!(close(a.at(r, c), b.at(r, c)), "{a:?}\n!=\n{b:?}");
            }
        }
    }

    // --- Vec3 ---

    #[test]
    fn add_sub_scale_negate() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_vec3(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_vec3(b - a, Vec3::new(3.0, 3.0, 3.0));
        assert_vec3(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert_vec3(-a, Vec3::new(-1.0, -2.0, -3.0));
    }

    #[test]
    fn dot_same_right_angle_opposite() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        assert!(close(x.dot(x), 1.0));
        assert!(close(x.dot(y), 0.0));
        assert!(close(x.dot(-x), -1.0));
    }

    #[test]
    fn cross_is_right_handed() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        assert_vec3(x.cross(y), Vec3::new(0.0, 0.0, 1.0));
        assert_vec3(y.cross(x), Vec3::new(0.0, 0.0, -1.0)); // order matters here too
    }

    #[test]
    fn length_and_normalise() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        assert!(close(v.length(), 5.0));
        assert!(close(v.normalise().length(), 1.0));
        assert_vec3(v.normalise(), Vec3::new(0.6, 0.8, 0.0));
    }

    #[test]
    fn normalise_zero_does_not_make_nan() {
        let n = Vec3::ZERO.normalise();
        assert!(!n.x.is_nan() && !n.y.is_nan() && !n.z.is_nan());
    }

    // --- Mat4: the cards ---

    #[test]
    fn look_at_from_the_front_is_the_old_fixed_camera() {
        // § 12–19's camera: 5 back along +z, looking at the origin = translation(0,0,−5).
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        );
        let old = Mat4::translation(Vec3::new(0.0, 0.0, -5.0));
        let p = Vec4::new(1.0, 2.0, 3.0, 1.0);
        assert_vec4(view * p, old * p);
    }

    #[test]
    fn look_at_puts_the_target_straight_ahead_and_keeps_up_up() {
        // A camera up and to the side, looking at a point that isn't the origin.
        let eye = Vec3::new(4.0, 3.0, -2.0);
        let target = Vec3::new(1.0, 1.0, 1.0);
        let view = Mat4::look_at(eye, target, Vec3::new(0.0, 1.0, 0.0));
        let distance = (target - eye).length();
        // The target lands straight ahead: x = y = 0, z = −distance.
        assert_vec3(
            (view * Vec4::point(target)).xyz(),
            Vec3::new(0.0, 0.0, -distance),
        );
        // The eye itself lands on the origin.
        assert_vec3((view * Vec4::point(eye)).xyz(), Vec3::ZERO);
        // A point just above the target is above it on screen (y > 0), not to the side.
        let above = (view * Vec4::point(target + Vec3::new(0.0, 0.5, 0.0))).xyz();
        assert!(above.y > 0.0 && close(above.x, 0.0), "{above:?}");
    }

    #[test]
    fn identity_changes_nothing() {
        let p = Vec4::new(1.0, 2.0, 3.0, 1.0);
        assert_vec4(Mat4::IDENTITY * p, p);
        let m = Mat4::translation(Vec3::new(5.0, 0.0, 0.0)) * Mat4::rotation_y(1.0);
        assert_mat(Mat4::IDENTITY * m, m);
        assert_mat(m * Mat4::IDENTITY, m);
    }

    #[test]
    fn translation_moves_points_not_directions() {
        let t = Mat4::translation(Vec3::new(5.0, 0.0, 0.0));
        // Stop ①: (3, 3, 3, w=1) → (8, 3, 3); (0, 0, −1, w=0) unchanged.
        assert_vec4(
            t * Vec4::point(Vec3::new(3.0, 3.0, 3.0)),
            Vec4::new(8.0, 3.0, 3.0, 1.0),
        );
        let arrow = Vec4::direction(Vec3::new(0.0, 0.0, -1.0));
        assert_vec4(t * arrow, arrow);
    }

    #[test]
    fn scale_stretches_each_axis() {
        let s = Mat4::scale(Vec3::new(2.0, 3.0, 4.0));
        assert_vec4(
            s * Vec4::point(Vec3::new(1.0, 1.0, 1.0)),
            Vec4::new(2.0, 3.0, 4.0, 1.0),
        );
    }

    #[test]
    fn quarter_turns_land_where_expected() {
        let x = Vec4::point(Vec3::new(1.0, 0.0, 0.0));
        let y = Vec4::point(Vec3::new(0.0, 1.0, 0.0));
        // Right-handed, counter-clockwise when the axis points at you.
        assert_vec4(
            Mat4::rotation_y(FRAC_PI_2) * x,
            Vec4::new(0.0, 0.0, -1.0, 1.0),
        );
        assert_vec4(
            Mat4::rotation_x(FRAC_PI_2) * y,
            Vec4::new(0.0, 0.0, 1.0, 1.0),
        );
        assert_vec4(
            Mat4::rotation_z(FRAC_PI_2) * x,
            Vec4::new(0.0, 1.0, 0.0, 1.0),
        );
    }

    #[test]
    fn full_turn_comes_back() {
        let p = Vec4::point(Vec3::new(1.0, 2.0, 3.0));
        assert_vec4(Mat4::rotation_y(2.0 * PI) * p, p);
    }

    #[test]
    fn degrees_to_radians() {
        assert!(close(90.0_f32.to_radians(), FRAC_PI_2));
    }

    // --- Order matters (Q-9.2 / Q-9.3) ---

    #[test]
    fn order_matters() {
        let t = Mat4::translation(Vec3::new(3.0, 0.0, 0.0));
        let r = Mat4::rotation_y(FRAC_PI_2);
        let centre = Vec4::point(Vec3::ZERO);
        // Turn, then move: spins in place, ends up at (3, 0, 0).
        assert_vec4(t * r * centre, Vec4::new(3.0, 0.0, 0.0, 1.0));
        // Move, then turn: swung round the origin to (0, 0, −3).
        assert_vec4(r * t * centre, Vec4::new(0.0, 0.0, -3.0, 1.0));
    }

    #[test]
    fn spin_around_own_centre_keeps_centre_still() {
        let c = Vec3::new(2.0, 1.0, 0.0);
        let model = Mat4::translation(c) * Mat4::rotation_y(1.234) * Mat4::translation(-c);
        assert_vec4(model * Vec4::point(c), Vec4::point(c));
        // Q-9.3: two cards → the centre ends up on the origin.
        let at_origin = Mat4::rotation_y(1.234) * Mat4::translation(-c);
        assert_vec4(at_origin * Vec4::point(c), Vec4::point(Vec3::ZERO));
    }

    // --- Perspective (Q-9.4) ---

    /// What the GPU does after the vertex shader: divide by w.
    fn divide(v: Vec4) -> Vec3 {
        Vec3::new(v.x / v.w, v.y / v.w, v.z / v.w)
    }

    #[test]
    fn perspective_near_is_depth_0_far_is_depth_1() {
        let (near, far) = (0.1, 100.0);
        let p = Mat4::perspective(FRAC_PI_2, 1.0, near, far);
        assert!(close(
            divide(p * Vec4::point(Vec3::new(0.0, 0.0, -near))).z,
            0.0
        ));
        assert!(close(
            divide(p * Vec4::point(Vec3::new(0.0, 0.0, -far))).z,
            1.0
        ));
    }

    #[test]
    fn twice_as_far_looks_half_as_big() {
        let p = Mat4::perspective(FRAC_PI_2, 1.0, 0.1, 100.0);
        let near_x = divide(p * Vec4::point(Vec3::new(1.0, 0.0, -2.0))).x;
        let far_x = divide(p * Vec4::point(Vec3::new(1.0, 0.0, -4.0))).x;
        assert!(close(far_x, near_x / 2.0));
    }

    #[test]
    fn perspective_flips_y_for_vulkan() {
        let p = Mat4::perspective(FRAC_PI_2, 1.0, 0.1, 100.0);
        // Up in the world (+y) must come out negative: Vulkan's y points down.
        assert!(divide(p * Vec4::point(Vec3::new(0.0, 1.0, -2.0))).y < 0.0);
    }

    #[test]
    fn aspect_squeezes_x_on_wide_windows() {
        let square = Mat4::perspective(FRAC_PI_2, 1.0, 0.1, 100.0);
        let wide = Mat4::perspective(FRAC_PI_2, 2.0, 0.1, 100.0);
        let p = Vec4::point(Vec3::new(1.0, 0.0, -2.0));
        assert!(close(divide(wide * p).x, divide(square * p).x / 2.0));
    }

    // --- Memory layout (Q-9.5) ---

    #[test]
    fn to_cols_array_gives_memory_order() {
        let t = Mat4::translation(Vec3::new(5.0, 6.0, 7.0));
        assert_eq!(t.to_cols_array()[3], [5.0, 6.0, 7.0, 1.0]);
    }

    #[test]
    fn storage_is_column_major() {
        let t = Mat4::translation(Vec3::new(5.0, 6.0, 7.0));
        // The move lives in the 4th COLUMN, which is cols[3] in memory.
        assert_eq!(t.cols[3], [5.0, 6.0, 7.0, 1.0]);
    }
}
