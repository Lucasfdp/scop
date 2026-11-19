//! scop — a `.obj` viewer built on the `engine` crate. scop only says *what* to
//! show and how keys behave; the engine does everything GPU- and window-related.

use std::env;
use std::process::ExitCode;

use engine::{
    App, DrawStyle, Flow, Frame, Gfx, GpuInfo, Image, Input, Key, Mat4, MeshData, MeshId, ObjModel,
    PipelineId, RunError, Shaders, TextureId, Vec3, Vertex, WindowConfig,
};

/// The mesh shaders, compiled by `build.rs` and embedded in the binary.
const MESH_SHADERS: Shaders = Shaders {
    vertex: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.vert.spv")),
    fragment: include_bytes!(concat!(env!("OUT_DIR"), "/mesh.frag.spv")),
};

/// The cheerful picture (§ 14), embedded in the binary so scop works from any
/// directory, used when no picture is given. Replace `resources/texture.tga` with
/// your own and rebuild.
const TEXTURE_TGA: &[u8] = include_bytes!("../../resources/texture.tga");

/// Automatic spin around y, in radians per second (about one turn every 8 s).
const SPIN_SPEED: f32 = 0.8;
/// Turning with the keys, in radians per second (about a quarter turn per second).
const TURN_SPEED: f32 = 1.5;
/// Moving with the keys, in model sizes (radii) per second (§ 13, Q-13.5).
const MOVE_SPEED: f32 = 0.2;
/// How long a full fade between grey and picture takes, in seconds (§ 15).
const FADE_SECONDS: f32 = 0.6;
/// Vertical field of view of the camera at the start (and after R). The camera's
/// distance is always worked out from this one, so zooming doesn't move it (§ 18).
const FOV_Y_DEGREES: f32 = 60.0;
/// Zoom limits (§ 18): near 0° the image blows up, near 180° it turns inside out.
const FOV_MIN_DEGREES: f32 = 10.0;
const FOV_MAX_DEGREES: f32 = 100.0;
/// Each wheel notch multiplies the field of view by this (0.9 = 10 % narrower).
const ZOOM_PER_NOTCH: f32 = 0.9;
/// Z / X held: how many notches' worth of zoom per second.
const ZOOM_KEY_NOTCHES_PER_SECOND: f32 = 6.0;
/// The draw styles V cycles through, in order (§ 18). Ones the GPU can't do are skipped.
const STYLES: [DrawStyle; 3] = [DrawStyle::Fill, DrawStyle::Wireframe, DrawStyle::Points];
/// Background colour (linear RGBA): very dark, so the greys stand out.
const BACKGROUND: [f32; 4] = [0.02, 0.02, 0.03, 1.0];

const CONTROLS: &str = "\
controls:
  arrows      move left / right / up / down
  W / S       move closer / further
  I / K       turn around x (tilt)
  J / L       turn around y (spin)
  U / O       turn around z (roll)
  T           fade between grey faces and the picture
  N           rotate which object gets which picture
  V           solid → wireframe → points
  wheel, Z/X  zoom in / out (field of view)
  Space       pause / resume the automatic spin
  R           reset (placement and zoom)
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

/// The grey ↔ picture fade (§ 15): a dimmer, not a light switch. T flips where it's
/// heading (`target`); every frame `progress` takes a small step towards it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Fade {
    /// 0 = heading to grey, 1 = heading to the picture.
    target: f32,
    /// How far along it is, 0..1, moving at a constant speed (before easing).
    progress: f32,
}

impl Fade {
    const GREY: Fade = Fade {
        target: 0.0,
        progress: 0.0,
    };

    /// T: turn the dimmer the other way. Only the target changes; `progress`
    /// carries on from where it is, so pressing mid-fade never jumps (Q-15.3).
    fn toggle(&mut self) {
        self.target = 1.0 - self.target;
    }

    /// One frame: move `progress` towards `target` by at most `dt / FADE_SECONDS`,
    /// never past it (that's the clamp), so it arrives and stays.
    fn step(&mut self, dt: f32) {
        let max_step = dt / FADE_SECONDS;
        let gap = self.target - self.progress;
        self.progress += gap.clamp(-max_step, max_step);
    }

    /// What the shader gets: the progress, eased with smoothstep (slow–fast–slow).
    fn blend(&self) -> f32 {
        smoothstep(self.progress)
    }
}

/// The classic easing curve `t² × (3 − 2t)`: 0 → 0, 0.5 → 0.5, 1 → 1, with zero
/// speed at both ends, so the fade starts and stops without a kick (Q-15.5).
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Zoom (§ 18): a new field of view after the wheel moved `notches` (+ = zoom in).
/// Multiplying, not adding, so every notch feels the same at any zoom level;
/// clamped, so the image can't blow up or turn inside out.
fn zoom(fov_degrees: f32, notches: f32) -> f32 {
    (fov_degrees * ZOOM_PER_NOTCH.powf(notches)).clamp(FOV_MIN_DEGREES, FOV_MAX_DEGREES)
}

/// scop's state. The GPU tickets (pipelines, meshes, textures) are empty until the
/// engine calls `setup`.
struct Scop {
    /// One mesh per object in the file (§ 18), ready to upload in `setup`.
    object_meshes: Vec<MeshData>,
    /// Middle of the model's bounding box: what it turns around (§ 12, Q-9.3).
    centre: Vec3,
    /// Size of the model: camera distance and movement speed scale with it.
    radius: f32,
    placement: Placement,
    fade: Fade,
    /// Vertical field of view in degrees: the zoom (§ 18).
    fov_y: f32,
    /// The decoded pictures, uploaded in `setup`.
    pictures: Vec<Image>,
    /// N: object `i` uses picture `(i + picture_shift) mod count` (§ 18).
    picture_shift: usize,
    /// One pipeline per draw style this GPU can do, in `STYLES` order.
    pipelines: Vec<(DrawStyle, PipelineId)>,
    /// Which entry of `pipelines` is in use (V).
    style: usize,
    meshes: Vec<MeshId>,
    textures: Vec<TextureId>,
}

impl App for Scop {
    fn setup(&mut self, gfx: &mut Gfx<'_>) -> Result<(), RunError> {
        let info = gfx.info();
        print_gpu_info(&info);
        // One pipeline per style, all from the same shaders (§ 18).
        for style in STYLES {
            if gfx.supports(style) {
                let pipeline = gfx.create_styled_pipeline(MESH_SHADERS, style)?;
                self.pipelines.push((style, pipeline));
            } else {
                println!("{style:?}: not supported by this GPU, V skips it");
            }
        }
        let (smallest, largest) = info.point_size_range;
        println!("point sizes: {smallest}..{largest} px");
        println!("{CONTROLS}");
        for mesh in &self.object_meshes {
            self.meshes.push(gfx.create_mesh(mesh)?);
        }
        for picture in &self.pictures {
            self.textures.push(gfx.create_texture(picture)?);
        }
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
            self.fov_y = FOV_Y_DEGREES;
        }
        if input.pressed(Key::N) {
            self.picture_shift += 1;
        }
        if input.pressed(Key::V) && !self.pipelines.is_empty() {
            self.style = (self.style + 1) % self.pipelines.len();
            println!("view: {:?}", self.pipelines[self.style].0);
        }

        if input.pressed(Key::T) {
            self.fade.toggle();
        }
        self.fade.step(dt);

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

        // Zoom: the wheel's notches this frame, plus Z / X as notches per second.
        let key_zoom =
            f32::from(u8::from(input.held(Key::Z))) - f32::from(u8::from(input.held(Key::X)));
        let notches = input.scroll() + key_zoom * ZOOM_KEY_NOTCHES_PER_SECOND * dt;
        self.fov_y = zoom(self.fov_y, notches);
        Flow::Continue
    }

    fn draw(&self, frame: &mut Frame) {
        frame.clear(BACKGROUND);

        // Framing: far enough back that a sphere of `radius` fits the view (§ 12).
        // Always from the *starting* field of view: if it used the zoomed one, the
        // camera would back off exactly as much as the zoom magnifies — no zoom at all.
        let distance = self.radius / (FOV_Y_DEGREES.to_radians() / 2.0).sin();
        let near = self.radius * 0.01; // as large as possible, for depth precision (§ 11)
        // Room for the model to be pushed back with − (it can go a few sizes away).
        let far = distance + self.radius * 20.0;

        // Camera, once per frame (the noticeboard). Aspect is read fresh every frame.
        // The zoom only changes the lens (§ 18): same camera, narrower view.
        let projection = Mat4::perspective(self.fov_y.to_radians(), frame.aspect(), near, far);
        let view = Mat4::translation(Vec3::new(0.0, 0.0, -distance));
        frame.set_camera(projection * view);

        // The model's placement (the sticky note): one for the whole file, so the
        // objects keep their places relative to each other.
        let model = self.placement.matrix(self.centre);
        let Some(&(_, pipeline)) = self.pipelines.get(self.style) else {
            return;
        };
        // One draw per object, each with its own picture (§ 18, Q-18.3).
        for (i, &mesh) in self.meshes.iter().enumerate() {
            if let Some(texture) = picture_for(i, self.picture_shift, &self.textures) {
                frame.draw_textured(pipeline, mesh, texture, model, self.fade.blend());
            }
        }
    }
}

/// Which picture object `object` uses: they take turns, and N shifts everyone on
/// by one. `None` only if there are no pictures at all.
/// (Generic only so the tests can use plain numbers: apps can't make `TextureId`s.)
fn picture_for<T: Copy>(object: usize, shift: usize, textures: &[T]) -> Option<T> {
    if textures.is_empty() {
        return None;
    }
    Some(textures[(object + shift) % textures.len()])
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
        eprintln!("usage: scop <file.obj> [picture.tga ...]");
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
        "{path}: {} corners, {} triangles, {} open edges, {} objects ({})",
        obj.positions.len(),
        obj.triangles.len(),
        obj.open_edges(), // 0 = sealed; more = the surface has gaps you can see past (§ 17)
        obj.object_names.len(),
        obj.object_names.join(", ")
    );

    // The pictures: every .tga after the model (§ 18), or the embedded one (§ 14).
    // Our own TGA loader either way; a broken picture is reported like a broken model.
    let mut pictures = Vec::new();
    for tga_path in &args[2..] {
        match std::fs::read(tga_path) {
            Ok(bytes) => match load_picture(tga_path, &bytes) {
                Some(picture) => pictures.push(picture),
                None => return ExitCode::FAILURE,
            },
            Err(err) => {
                eprintln!("scop: {tga_path}: {err}");
                return ExitCode::FAILURE;
            }
        }
    }
    if pictures.is_empty() {
        match load_picture("resources/texture.tga (embedded)", TEXTURE_TGA) {
            Some(picture) => pictures.push(picture),
            None => return ExitCode::FAILURE,
        }
    }

    let config = WindowConfig {
        title: String::from("scop"),
        width: 1280,
        height: 720,
    };
    let scop = Scop {
        object_meshes: object_meshes(&obj),
        pictures,
        picture_shift: 0,
        centre: obj.centre(),
        // A model that's a single point would put the camera on top of it.
        radius: obj.radius().max(0.001),
        placement: Placement::START,
        fade: Fade::GREY,
        fov_y: FOV_Y_DEGREES,
        pipelines: Vec::new(),
        style: 0,
        meshes: Vec::new(),
        textures: Vec::new(),
    };
    match engine::run(config, scop) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("scop: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Decodes one picture with our own TGA loader, reporting it (or what's wrong with it).
fn load_picture(name: &str, bytes: &[u8]) -> Option<Image> {
    match engine::load_tga(bytes) {
        Ok(picture) => {
            println!("{name}: {}x{} picture", picture.width, picture.height);
            Some(picture)
        }
        Err(err) => {
            eprintln!("scop: {name}: {err}");
            None
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

/// One mesh per object in the file (§ 18), each with one flat grey per `.obj` face
/// (§ 13): every triangle gets its own three corners (duplicated, not shared), all
/// the colour of its face, so nothing blends across faces. Indices become 0, 1, 2, …
/// — each corner used once. Duplicated corners also let two faces that share a corner
/// give it different UVs, which box mapping needs.
fn object_meshes(obj: &ObjModel) -> Vec<MeshData> {
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
                let grey = face_grey(face);
                let normal = normals[face as usize];
                for &corner in triangle {
                    let p = obj.positions[corner as usize];
                    vertices.push(Vertex {
                        position: [p.x, p.y, p.z],
                        colour: [grey, grey, grey],
                        uv: box_uv(p, normal, min, max),
                    });
                }
            }
            let indices = (0..vertices.len() as u32).collect();
            Some(MeshData { vertices, indices })
        })
        .collect()
}

/// Which way each `.obj` face looks (index = face number). A face's triangles each
/// add their cross product: together that's the face's Newell normal (§ 17), so a
/// quad's two triangles always agree — no seam through the middle of a face.
/// Only the direction matters here, so it isn't normalised.
fn face_normals(obj: &ObjModel) -> Vec<Vec3> {
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
fn main_axis(n: Vec3) -> usize {
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
///   picture's squares stay square (Q-18.2). The shader corrects for the picture's
///   own width ÷ height.
/// - Each projector is seen from the side the face looks out of: "right" on the
///   picture is the viewer's right, so a face looking the negative way flips u
///   (otherwise it'd be mirrored), and "up" stays +y on the side views.
fn box_uv(p: Vec3, normal: Vec3, min: Vec3, max: Vec3) -> [f32; 2] {
    let size = max - min;
    let scale = size.x.max(size.y).max(size.z);
    // A model that's a single point: any UV will do.
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let from_min = (p - min) * (1.0 / scale); // 0 at min .. ≤ 1 at max
    let from_max = (max - p) * (1.0 / scale); // 0 at max .. ≤ 1 at min
    match main_axis(normal) {
        // Looking along x (a side view): z runs across, y down the picture.
        0 if normal.x >= 0.0 => [from_max.z, from_max.y], // seen from +x: right is −z
        0 => [from_min.z, from_max.y],                    // seen from −x: right is +z
        // Looking along y (top or bottom): x runs across, z down the picture.
        1 if normal.y >= 0.0 => [from_min.x, from_min.z], // top: right is +x
        1 => [from_max.x, from_min.z],                    // bottom: flipped
        // Looking along z (front or back): x runs across, y down the picture.
        _ if normal.z >= 0.0 => [from_min.x, from_max.y], // front: right is +x
        _ => [from_max.x, from_max.y],                    // back: right is −x
    }
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
        let meshes = object_meshes(&obj);
        assert_eq!(meshes.len(), 1); // no `o` lines: one object
        let mesh = &meshes[0];
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
    fn main_axis_is_the_largest_part_of_the_normal() {
        assert_eq!(main_axis(Vec3::new(0.9, 0.3, -0.2)), 0);
        assert_eq!(main_axis(Vec3::new(0.1, -0.8, 0.5)), 1);
        assert_eq!(main_axis(Vec3::new(0.0, 0.0, -1.0)), 2);
        assert_eq!(main_axis(Vec3::new(1.0, 1.0, 1.0)), 0); // ties → earlier axis
    }

    #[test]
    fn u_and_v_share_one_scale() {
        // A long box, 4 wide, 1 tall, seen from the front: the picture's squares stay
        // square, so v covers a quarter of what u covers (not both 0..1).
        let (min, max) = (Vec3::ZERO, Vec3::new(4.0, 1.0, 0.0));
        let front = Vec3::new(0.0, 0.0, 1.0);
        let top_left = box_uv(Vec3::new(0.0, 1.0, 0.0), front, min, max);
        let bottom_right = box_uv(Vec3::new(4.0, 0.0, 0.0), front, min, max);
        assert_eq!(top_left, [0.0, 0.0]);
        assert_eq!(bottom_right, [1.0, 0.25]);
    }

    #[test]
    fn no_face_is_mirrored_and_up_stays_up() {
        // For each way a face can look: moving to the *viewer's* right must increase
        // u (not mirrored), and on the side views moving up must decrease v (v = 0
        // is the top of the picture).
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
        // A streak = a face whose UVs have (almost) no area: the whole face reads one
        // thin line of the picture. With box mapping each face keeps at least 1/√3 of
        // its area (its normal's largest part is ≥ 1/√3 of the whole), divided by the
        // object's scale². § 14's single projector broke this on the logo's sides.
        for name in [
            "../resources/42.obj",
            "../resources/teapot.obj",
            "../resources/test/concave.obj",
        ] {
            let obj = engine::parse_obj(&std::fs::read_to_string(name).unwrap()).unwrap();
            for (object, mesh) in object_meshes(&obj).iter().enumerate() {
                let (min, max) = obj.object_box(object as u32).unwrap();
                let size = max - min;
                let scale = size.x.max(size.y).max(size.z);
                for corners in mesh.vertices.chunks_exact(3) {
                    let corners: &[Vertex; 3] = corners.try_into().unwrap();
                    let p = corners.map(|v| Vec3::new(v.position[0], v.position[1], v.position[2]));
                    let uv = corners.map(|v| Vec3::new(v.uv[0], v.uv[1], 0.0));
                    let surface = area(p[0], p[1], p[2]) / (scale * scale);
                    let picture = area(uv[0], uv[1], uv[2]);
                    assert!(
                        picture >= surface / 3f32.sqrt() * 0.99,
                        "{name}: a triangle streaks ({picture} < {surface} / √3)"
                    );
                }
            }
        }
    }

    #[test]
    fn each_object_gets_its_own_mesh_fitted_to_its_own_box() {
        let text = std::fs::read_to_string("../resources/test/concave.obj").unwrap();
        let obj = engine::parse_obj(&text).unwrap();
        let meshes = object_meshes(&obj);
        // L prism 20 triangles, star prism 36 — 3 corners each.
        let corners: Vec<usize> = meshes.iter().map(|m| m.vertices.len()).collect();
        assert_eq!(corners, vec![60, 108]);
        for mesh in &meshes {
            // Fitted to *its* box: some UV reaches the far edge (1.0), none goes past.
            let largest = mesh
                .vertices
                .iter()
                .flat_map(|v| v.uv)
                .fold(0.0_f32, f32::max);
            assert!((largest - 1.0).abs() < 1e-5, "largest UV {largest}");
        }
    }

    #[test]
    fn pictures_take_turns_and_n_rotates_them() {
        let two = ['a', 'b'];
        let for_objects = |shift| -> Vec<_> {
            (0..3)
                .map(|i| picture_for(i, shift, &two).unwrap())
                .collect()
        };
        assert_eq!(for_objects(0), vec![two[0], two[1], two[0]]);
        assert_eq!(for_objects(1), vec![two[1], two[0], two[1]]); // N once
        assert_eq!(for_objects(2), for_objects(0)); // N twice: back where it started
        assert_eq!(picture_for(5, 3, &two[..1]), Some(two[0])); // one picture: everyone
        assert_eq!(picture_for::<char>(0, 0, &[]), None);
    }

    #[test]
    fn zoom_multiplies_and_clamps() {
        assert!((zoom(60.0, 1.0) - 54.0).abs() < 1e-4); // one notch: 10 % narrower
        assert!((zoom(zoom(60.0, 1.0), -1.0) - 60.0).abs() < 1e-4); // and back
        assert_eq!(zoom(60.0, 100.0), FOV_MIN_DEGREES); // never reaches 0°
        assert_eq!(zoom(60.0, -100.0), FOV_MAX_DEGREES); // never reaches 180°
        assert_eq!(zoom(60.0, 0.0), 60.0); // wheel still: no change
    }

    #[test]
    fn embedded_texture_loads() {
        let picture = engine::load_tga(TEXTURE_TGA).unwrap();
        assert!(picture.width > 0 && picture.height > 0);
        assert_eq!(
            picture.rgba.len(),
            (picture.width * picture.height * 4) as usize
        );
    }

    #[test]
    fn fade_arrives_and_stays() {
        let mut fade = Fade::GREY;
        fade.toggle(); // heading to the picture
        for _ in 0..100 {
            fade.step(0.016); // 100 frames ≈ 1.6 s > 0.6 s
        }
        assert_eq!(fade.progress, 1.0); // arrived, not overshot
        assert_eq!(fade.blend(), 1.0);
    }

    #[test]
    fn fade_reverses_mid_way_without_a_jump() {
        let mut fade = Fade::GREY;
        fade.toggle();
        fade.step(FADE_SECONDS * 0.4); // 40 % of the way
        let before = fade.progress;
        fade.toggle(); // T again
        assert_eq!(fade.progress, before); // pressing changed only the target
        fade.step(0.016);
        assert!(fade.progress < before && fade.progress > before - 0.05); // small step back
    }

    #[test]
    fn smoothstep_keeps_ends_and_middle() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(0.5), 0.5);
        assert_eq!(smoothstep(1.0), 1.0);
        assert!(smoothstep(0.1) < 0.1); // slow start
        assert!(smoothstep(0.9) > 0.9); // slow finish (already nearly there)
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
