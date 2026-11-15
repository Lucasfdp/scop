//! scop — a `.obj` viewer built on the `engine` crate. scop only says *what* to
//! show and how keys behave; the engine does everything GPU- and window-related.

use std::env;
use std::process::ExitCode;

use engine::{
    App, Flow, Frame, Gfx, GpuInfo, Input, Key, Mat4, MeshData, MeshId, ObjModel, PipelineId,
    RunError, Shaders, Vec3, Vertex, WindowConfig,
};

/// The mesh shaders, compiled by `build.rs` and embedded in the binary.
const MESH_SHADERS: Shaders = Shaders {
    vertex: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.vert.spv")),
    fragment: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.frag.spv")),
};

/// Automatic spin around y, in radians per second (about one turn every 8 s).
const SPIN_SPEED: f32 = 0.8;
/// Turning with the keys, in radians per second (about a quarter turn per second).
const TURN_SPEED: f32 = 1.5;
/// Moving with the keys, in model sizes (radii) per second (§ 13, Q-13.5).
const MOVE_SPEED: f32 = 0.2;
/// Vertical field of view of the camera.
const FOV_Y_DEGREES: f32 = 60.0;
/// Background colour (linear RGBA): very dark, so the greys stand out.
const BACKGROUND: [f32; 4] = [0.02, 0.02, 0.03, 1.0];

const CONTROLS: &str = "\
controls:
  arrows      move left / right / up / down
  W / S       move closer / further
  I / K       turn around x (tilt)
  J / L       turn around y (spin)
  U / O       turn around z (roll)
  Space       pause / resume the automatic spin
  R           reset
  Esc         quit";

/// Where the model is and how it's turned (§ 13: *model state*). Keys change these
/// numbers in `update`; `draw` builds the model matrix from them.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placement {
    position: Vec3,
    /// Turns around x, y, z, in radians (Euler angles).
    angles: Vec3,
    /// The automatic spin around y, in radians.
    spin: f32,
    spinning: bool,
}

impl Placement {
    const START: Placement = Placement {
        position: Vec3::ZERO,
        angles: Vec3::ZERO,
        spin: 0.0,
        spinning: true,
    };

    /// The § 9 recipe, read right to left: centre it on the origin, roll (z), tilt (x),
    /// spin (y), then move it. Moving is leftmost, so the arrows always follow the
    /// screen's axes, whatever way the model is turned (Q-13.4).
    fn matrix(&self, centre: Vec3) -> Mat4 {
        Mat4::translation(self.position)
            * Mat4::rotation_y(self.spin + self.angles.y)
            * Mat4::rotation_x(self.angles.x)
            * Mat4::rotation_z(self.angles.z)
            * Mat4::translation(-centre)
    }
}

/// scop's state. The ids are `None` until the engine calls `setup`.
struct Scop {
    /// The model's vertices and triangles, ready to upload in `setup`.
    mesh: MeshData,
    /// Middle of the model's bounding box: what it turns around (§ 12, Q-9.3).
    centre: Vec3,
    /// Size of the model: camera distance and movement speed scale with it.
    radius: f32,
    placement: Placement,
    pipeline: Option<PipelineId>,
    model: Option<MeshId>,
}

impl App for Scop {
    fn setup(&mut self, gfx: &mut Gfx<'_>) -> Result<(), RunError> {
        print_gpu_info(&gfx.info());
        println!("{CONTROLS}");
        self.pipeline = Some(gfx.create_pipeline(MESH_SHADERS)?);
        self.model = Some(gfx.create_mesh(&self.mesh)?);
        Ok(())
    }

    fn update(&mut self, input: &Input, dt: f32) -> Flow {
        if input.pressed(Key::Escape) {
            return Flow::Exit;
        }
        let p = &mut self.placement;

        // Toggles: `pressed`, so one press = one switch (Q-13.3).
        if input.pressed(Key::Space) {
            p.spinning = !p.spinning;
        }
        if input.pressed(Key::R) {
            *p = Placement::START;
        }

        // Continuous: `held`, × dt so the speed is per second at any frame rate.
        let step = MOVE_SPEED * self.radius * dt;
        p.position = p.position
            + axis_input(
                input,
                (Key::Left, Key::Right),
                (Key::Down, Key::Up),
                (Key::S, Key::W),
            ) * step;
        p.angles = p.angles
            + axis_input(input, (Key::K, Key::I), (Key::J, Key::L), (Key::O, Key::U))
                * (TURN_SPEED * dt);
        if p.spinning {
            p.spin += SPIN_SPEED * dt;
        }
        Flow::Continue
    }

    fn draw(&self, frame: &mut Frame) {
        frame.clear(BACKGROUND);

        // Framing: far enough back that a sphere of `radius` fits the view (§ 12).
        let fov_y = FOV_Y_DEGREES.to_radians();
        let distance = self.radius / (fov_y / 2.0).sin();
        let near = self.radius * 0.01; // as large as possible, for depth precision (§ 11)
        // Room for the model to be pushed back with − (it can go a few sizes away).
        let far = distance + self.radius * 20.0;

        // Camera, once per frame (the noticeboard). Aspect is read fresh every frame.
        let projection = Mat4::perspective(fov_y, frame.aspect(), near, far);
        let view = Mat4::translation(Vec3::new(0.0, 0.0, -distance));
        frame.set_camera(projection * view);

        // The model's placement (the sticky note).
        let model = self.placement.matrix(self.centre);
        if let (Some(pipeline), Some(mesh)) = (self.pipeline, self.model) {
            frame.draw(pipeline, mesh, model);
        }
    }
}

/// −1, 0 or +1 for each axis from a (negative key, positive key) pair per axis.
/// Both keys held = they cancel out.
fn axis_input(input: &Input, x: (Key, Key), y: (Key, Key), z: (Key, Key)) -> Vec3 {
    let axis = |(minus, plus): (Key, Key)| -> f32 {
        f32::from(u8::from(input.held(plus))) - f32::from(u8::from(input.held(minus)))
    };
    Vec3::new(axis(x), axis(y), axis(z))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    let Some(path) = args.get(1) else {
        eprintln!("usage: scop <file.obj>");
        return ExitCode::FAILURE;
    };

    let text = match engine::read_file(path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("scop: {path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    let obj = match engine::parse_obj(&text) {
        Ok(obj) => obj,
        Err(err) => {
            eprintln!("scop: {path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "{path}: {} corners, {} triangles",
        obj.positions.len(),
        obj.triangles.len()
    );

    let config = WindowConfig {
        title: String::from("scop"),
        width: 1280,
        height: 720,
    };
    let scop = Scop {
        mesh: grey_faces(&obj),
        centre: obj.centre(),
        // A model that's a single point would put the camera on top of it.
        radius: obj.radius().max(0.001),
        placement: Placement::START,
        pipeline: None,
        model: None,
    };
    match engine::run(config, scop) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("scop: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Printed once the window and the GPU are set up (§ 3, § 4 and § 11 checkpoints).
fn print_gpu_info(info: &GpuInfo) {
    let (major, minor, patch) = info.vulkan_version;
    let (width, height) = info.extent;
    println!(
        "GPU: {} (Vulkan {major}.{minor}.{patch}), queue family {}",
        info.name, info.queue_family
    );
    println!(
        "swapchain: {}, {}, {width}x{height}, {} images",
        info.swapchain_format, info.present_mode, info.image_count
    );
    println!("depth buffer: {}", info.depth_format);
}

/// The model with one flat grey per `.obj` face (§ 13): every triangle gets its own
/// three corners (duplicated, not shared), all the colour of its face, so nothing
/// blends across faces. Indices become 0, 1, 2, … — each corner used once.
fn grey_faces(obj: &ObjModel) -> MeshData {
    let mut vertices = Vec::with_capacity(obj.triangles.len() * 3);
    for (triangle, &face) in obj.triangles.iter().zip(&obj.triangle_faces) {
        let grey = face_grey(face);
        for &corner in triangle {
            let p = obj.positions[corner as usize];
            vertices.push(Vertex {
                position: [p.x, p.y, p.z],
                colour: [grey, grey, grey],
            });
        }
    }
    let indices = (0..vertices.len() as u32).collect();
    MeshData { vertices, indices }
}

/// A subtle grey for face `index`: always in 0.25..0.75 (never black or white), and
/// consecutive faces land far apart because 0.618… (the golden ratio's fraction)
/// never lines up with whole steps.
fn face_grey(index: u32) -> f32 {
    let step = (index as f32 * 0.618_034).fract();
    0.25 + 0.5 * step
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greys_are_subtle_and_neighbours_differ() {
        for i in 0..1000 {
            let g = face_grey(i);
            assert!((0.25..0.75).contains(&g), "face {i}: {g}");
            assert!(
                (g - face_grey(i + 1)).abs() > 0.05,
                "faces {i} and {}",
                i + 1
            );
        }
    }

    #[test]
    fn every_triangle_gets_its_own_corners() {
        let obj = engine::parse_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
        let mesh = grey_faces(&obj);
        // A quad → 2 triangles → 6 corners (not the 4 shared ones), indices 0..6.
        assert_eq!(mesh.vertices.len(), 6);
        assert_eq!(mesh.indices, vec![0, 1, 2, 3, 4, 5]);
        // Same face → same grey on all six corners.
        assert!(
            mesh.vertices
                .iter()
                .all(|v| v.colour == mesh.vertices[0].colour)
        );
    }

    #[test]
    fn translation_is_applied_after_the_turn() {
        // Q-13.4: turned 90° around y, then moved right → still right on screen.
        let mut p = Placement::START;
        p.angles.y = std::f32::consts::FRAC_PI_2;
        p.position = Vec3::new(1.0, 0.0, 0.0);
        let centre_moved_to = p.matrix(Vec3::ZERO) * engine::Vec4::point(Vec3::ZERO);
        assert!((centre_moved_to.x - 1.0).abs() < 1e-5 && centre_moved_to.z.abs() < 1e-5);
    }
}
