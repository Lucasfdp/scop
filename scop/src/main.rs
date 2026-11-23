//! scop — a `.obj` viewer built on the `engine` crate. Since § 24 scop holds only the
//! subject's and scop's own choices: the keys, the grey faces, which picture each
//! object gets, the crowd demo, the title. Everything reusable — camera, transforms,
//! fades, mesh building, shaders, hot reload, the sky — is the engine's.

use std::env;
use std::path::Path;
use std::process::ExitCode;

use engine::{
    App, CameraSettings, ColourMode, DefaultParams, DrawStyle, Easing, Flow, FpsCounter,
    FpsReading, Frame, Gfx, GpuInfo, Image, Input, InstanceRange, Key, LoadedObj, Mat4, MeshData,
    MeshId, MouseButton, OrbitCamera, PipelineId, RunError, ShaderWatch, Shaders, Sky, TextureId,
    Transform, Tween, UvMapping, Vec3, Vertex, WindowConfig,
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
/// Z / X held: how many wheel notches' worth of zoom per second (§ 18).
const ZOOM_KEY_NOTCHES_PER_SECOND: f32 = 6.0;
/// The light (§ 20): the way it shines — from above, front-left, towards the model.
/// Fixed in the world: orbiting doesn't move it (Q-20.1). The generated sky puts its
/// sun the opposite way, where the light comes from (Q-23.5).
const LIGHT_DIRECTION: Vec3 = Vec3::new(1.0, -1.0, -1.0);
/// The crowd (§ 22, scop's demo): copies per side of the grid — 8 × 8 × 8 = 512.
const CROWD_SIDE: usize = 8;
/// Each copy's size as a share of its grid cell (1.0 = neighbours would touch).
const CROWD_COPY_SIZE: f32 = 0.8;
/// How often the FPS in the title is updated, in seconds (§ 19, Q-19.2).
const FPS_WINDOW: f32 = 0.5;
/// The draw styles V cycles through, in order (§ 18). Ones the GPU can't do are skipped.
const STYLES: [DrawStyle; 3] = [DrawStyle::Fill, DrawStyle::Wireframe, DrawStyle::Points];
/// Background colour (linear RGBA).
const BACKGROUND: [f32; 4] = [0.02, 0.02, 0.03, 1.0];

// ASCII only: the built-in font has no arrows (anything else is drawn as `?`).
const CONTROLS: &str = "\
controls:
  arrow keys  move left / right / up / down
  W / S       move closer / further
  I / K       turn around x (tilt)
  J / L       turn around y (spin)
  U / O       turn around z (roll)
  T           fade between grey faces and the picture
  N           rotate which object gets which picture
  V           solid -> wireframe -> points
  C           colours: normal -> facing -> depth -> chrome
  M           chrome: live reflections <-> fixed sky
  E           sky on / off
  wheel, Z/X  zoom in / out (field of view)
  mouse drag  orbit the camera round the model
  B           light on / off
  G           one model <-> a crowd of 512 (instancing)
  Space       pause / resume the automatic spin
  R           reset (placement, zoom and camera)
  P           props: coloured cubes orbiting the model (see them in live chrome)
  H           hide / show these controls
  Esc         quit
  drop a .obj on the window to open it
  save the engine's shaders while scop runs: they reload (hot reload)";

/// P: the props' colours (linear RGB), one cube each — bright, so they stand out in
/// chrome. How far out they orbit and how big they are, × the model's radius.
const PROP_COLOURS: [[f32; 3]; 6] = [
    [0.9, 0.1, 0.1],
    [0.95, 0.6, 0.05],
    [0.9, 0.9, 0.1],
    [0.1, 0.8, 0.2],
    [0.1, 0.4, 0.95],
    [0.6, 0.15, 0.9],
];
const PROP_ORBIT: f32 = 1.6;
const PROP_SIZE: f32 = 0.18;
/// Orbit speed, radians per second.
const PROP_SPEED: f32 = 0.5;

/// The on-screen controls' text colour and panel colour (linear RGB).
const TEXT_COLOUR: [f32; 3] = [0.9, 0.9, 0.9];
const PANEL_COLOUR: [f32; 3] = [0.01, 0.01, 0.015];

/// What scop prints for each colour mode (C).
fn describe(mode: ColourMode) -> &'static str {
    match mode {
        ColourMode::Normal => "normal",
        ColourMode::Facing => "facing (blue front, red back)",
        ColourMode::Depth => "depth (near white, far dark)",
        ColourMode::Chrome => "chrome (reflects what's around it; M: live / fixed)",
    }
}

/// scop's state. The GPU tickets (pipelines, meshes, textures) are empty until the
/// engine calls `setup`.
struct Scop {
    /// The model being shown. Its meshes are uploaded in `setup`, and replaced when a
    /// file is dropped (§ 19).
    model: LoadedObj,
    /// Where the model is and how it's turned by the keys (§ 13).
    placement: Transform,
    /// The automatic spin around y, added on top of `placement` (Space pauses it).
    spin: f32,
    spinning: bool,
    /// Grey ↔ picture (§ 15, T).
    fade: Tween,
    /// The orbit camera and zoom (§ 18, § 20).
    camera: OrbitCamera,
    /// The decoded pictures, uploaded in `setup`.
    pictures: Vec<Image>,
    /// N: object `i` uses picture `(i + picture_shift) mod count` (§ 18).
    picture_shift: usize,
    /// One default pipeline per draw style this GPU can do, in `STYLES` order.
    pipelines: Vec<(DrawStyle, PipelineId)>,
    /// Which entry of `pipelines` is in use (V).
    style: usize,
    meshes: Vec<MeshId>,
    textures: Vec<TextureId>,
    /// § 19: frames per second, for the title.
    fps: FpsCounter,
    fps_reading: Option<FpsReading>,
    /// C (§ 19, § 23).
    colour_mode: ColourMode,
    /// B (§ 20). Off at the start, so the mandatory greys look exactly as before.
    light_on: bool,
    /// § 21: watching the engine's default shaders; `None` = hot reload off.
    shader_watch: Option<ShaderWatch>,
    /// § 22: G — draw a crowd of copies instead of the model.
    crowd: bool,
    /// Seconds the crowd has been spinning.
    crowd_time: f32,
    /// § 23: the sky's six faces until `setup` uploads them, then the sky itself.
    sky_faces: Option<[Image; 6]>,
    sky: Option<Sky>,
    /// E. Off at the start, so the mandatory part looks exactly as before.
    sky_on: bool,
    /// § 25, M: chrome reflects a live probe (true) or the fixed sky (false).
    live_reflections: bool,
    /// § 25 extra, H: the controls on screen. On at the start.
    show_controls: bool,
    /// § 25 extra, P: coloured cubes orbiting the model, so live chrome has something
    /// to reflect besides the sky. Off at the start. One mesh per colour.
    props_on: bool,
    prop_meshes: Vec<MeshId>,
    /// Seconds the props have been orbiting.
    props_time: f32,
}

/// A subtle grey for face `index` (§ 13, the subject's distinguishable faces): always
/// in 0.25..0.75 (never black or white), and consecutive faces land far apart because
/// 0.618… (the golden ratio's fraction) never lines up with whole steps. scop's rule,
/// handed to the engine's mesh builder as the colour callback (§ 24).
fn face_grey(index: u32) -> [f32; 3] {
    let step = (index as f32 * 0.618_034).fract();
    [0.25 + 0.5 * step; 3]
}

/// Loads `path` with scop's choices: box-mapped UVs, grey faces. Prints the model's
/// stats on success; the error is returned for the caller to report.
fn load_model(path: &Path) -> Result<LoadedObj, String> {
    let model = engine::load_obj(path, UvMapping::Box, face_grey).map_err(|err| err.to_string())?;
    let objects = model.object_names.len();
    println!(
        "{}: {} corners, {} triangles, {} open edges, {objects} {} ({})",
        path.display(),
        model.corners,
        model.triangles,
        model.open_edges, // 0 = sealed; more = the surface has gaps you can see past (§ 17)
        if objects == 1 { "object" } else { "objects" },
        model.object_names.join(", ")
    );
    Ok(model)
}

/// The crowd's copies (§ 22, scop's demo): a `CROWD_SIDE`³ grid, each copy shrunk and
/// turning its own way, all fitting inside the model's sphere (`centre`, `radius`) — so
/// the camera, zoom and depth range need no change. Right to left: move the model's
/// centre to the origin, shrink, tilt, spin, move to its cell.
/// A cube of side 2 centred on the origin, one `colour`: 4 corners per face so each
/// face has its own normal (flat, like the models, § 20).
fn prop_cube(colour: [f32; 3]) -> MeshData {
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for axis in 0..3 {
        for sign in [-1.0, 1.0] {
            let mut normal = [0.0; 3];
            normal[axis] = sign;
            let first = vertices.len() as u32;
            for (a, b) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let mut position = [0.0; 3];
                position[axis] = sign;
                position[(axis + 1) % 3] = a;
                position[(axis + 2) % 3] = b;
                vertices.push(Vertex {
                    position,
                    colour,
                    uv: [(a + 1.0) / 2.0, (b + 1.0) / 2.0],
                    normal,
                });
            }
            indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
        }
    }
    MeshData { vertices, indices }
}

/// Where each prop is at `time`, in the world: evenly spaced on a ring round the
/// model's centre (`middle`), alternately above and below, each tumbling.
fn prop_places(middle: Vec3, radius: f32, time: f32) -> Vec<Mat4> {
    let count = PROP_COLOURS.len();
    let size = PROP_SIZE * radius;
    (0..count)
        .map(|i| {
            let angle = time * PROP_SPEED + i as f32 * std::f32::consts::TAU / count as f32;
            let height = if i % 2 == 0 { 0.35 } else { -0.35 } * radius;
            let offset = Vec3::new(angle.cos(), 0.0, angle.sin()) * (PROP_ORBIT * radius)
                + Vec3::new(0.0, height, 0.0);
            Mat4::translation(middle + offset)
                * Mat4::rotation_y(time + i as f32)
                * Mat4::rotation_x(time * 0.7)
                * Mat4::scale(Vec3::new(size, size, size))
        })
        .collect()
}

fn crowd_copies(centre: Vec3, radius: f32, time: f32) -> Vec<Mat4> {
    let side = CROWD_SIDE as f32;
    // Cell size: chosen so the whole grid, corners included, fits in the sphere.
    let cell = 2.0 * radius / (3f32.sqrt() * side);
    let size = CROWD_COPY_SIZE / (3f32.sqrt() * side); // each copy's size vs the model
    let middle = (side - 1.0) / 2.0;
    let mut copies = Vec::with_capacity(CROWD_SIDE.pow(3));
    for x in 0..CROWD_SIDE {
        for y in 0..CROWD_SIDE {
            for z in 0..CROWD_SIDE {
                let i = copies.len();
                let place = Vec3::new(x as f32 - middle, y as f32 - middle, z as f32 - middle);
                // Variety from the copy's number: its own speed, start angle and tilt.
                let speed = 0.4 + (i % 7) as f32 * 0.2;
                let spin = time * speed + i as f32;
                let tilt = (i % 5) as f32 * 0.3;
                copies.push(
                    Mat4::translation(centre + place * cell)
                        * Mat4::rotation_y(spin)
                        * Mat4::rotation_x(tilt)
                        * Mat4::scale(Vec3::new(size, size, size))
                        * Mat4::translation(-centre),
                );
            }
        }
    }
    copies
}

impl Scop {
    /// § 25 extra: the controls in the top-left corner on a dark panel (H hides
    /// them). Scale: one font texel per pixel up to 720 pixels tall, then bigger, so
    /// the text stays readable on a Retina screen.
    fn draw_controls(&self, frame: &mut Frame) {
        if !self.show_controls {
            return;
        }
        let scale = (frame.size().1 / 720.0).floor().max(1.0);
        let margin = 8.0 * scale;
        let (width, height) = engine::text_size(CONTROLS, scale);
        frame.draw_panel(
            (0.0, 0.0),
            (width + 2.0 * margin, height + 2.0 * margin),
            PANEL_COLOUR,
        );
        frame.draw_text(CONTROLS, (margin, margin), scale, TEXT_COLOUR);
    }

    /// § 21: a shader was saved. `compiled` = the four files in `default_shader_files`
    /// order. Swap every model pipeline, then the sky's; any error is printed and the
    /// old shaders keep drawing.
    fn reload_shaders(&mut self, compiled: &[Vec<u8>], gfx: &mut Gfx<'_>) {
        let [vertex, fragment, sky_vertex, sky_fragment] = compiled else {
            return;
        };
        let shaders = Shaders { vertex, fragment };
        for &(style, pipeline) in &self.pipelines {
            if let Err(err) = gfx.replace_pipeline(pipeline, shaders, style) {
                eprintln!("scop: shaders not reloaded ({style:?}): {err}");
                return;
            }
        }
        if let Some(sky) = self.sky {
            let shaders = Shaders {
                vertex: sky_vertex,
                fragment: sky_fragment,
            };
            // It stays a background pipeline (§ 23).
            if let Err(err) = gfx.replace_pipeline(sky.pipeline, shaders, DrawStyle::Fill) {
                eprintln!("scop: sky shaders not reloaded: {err}");
                return;
            }
        }
        println!("shaders reloaded");
    }

    /// A file was dropped on the window (§ 19). Anything that goes wrong is printed,
    /// and the current model stays: a bad drop never closes the viewer.
    fn open_dropped(&mut self, path: &Path, gfx: &mut Gfx<'_>) {
        let is_obj = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("obj"));
        if !is_obj {
            eprintln!("scop: {}: drop a .obj file", path.display());
            return;
        }
        let swapped = load_model(path).and_then(|model| {
            self.swap_model(model, gfx)
                .map_err(|err| format!("{}: {err}", path.display()))
        });
        if let Err(err) = swapped {
            eprintln!("scop: {err}");
        }
    }

    /// Uploads `model`'s meshes, then destroys the old ones (Q-19.3). New first: if an
    /// upload fails, the old model is still complete and stays on screen.
    fn swap_model(&mut self, mut model: LoadedObj, gfx: &mut Gfx<'_>) -> Result<(), RunError> {
        let mut new_meshes = Vec::new();
        for mesh in &model.meshes {
            match gfx.create_mesh(mesh) {
                Ok(id) => new_meshes.push(id),
                Err(err) => {
                    // Undo the half-done upload. Errors here are secondary: report the first.
                    for id in new_meshes {
                        let _ = gfx.destroy_mesh(id);
                    }
                    return Err(err);
                }
            }
        }
        // Each destroy waits for the GPU (frames in flight may still draw the old
        // model); the first wait drains it, so the others return straight away.
        for id in std::mem::replace(&mut self.meshes, new_meshes) {
            gfx.destroy_mesh(id)?;
        }
        model.meshes = Vec::new(); // uploaded: the CPU copy isn't needed any more
        self.model = model;
        // A new model starts like scop does: placed, unzoomed. Pictures, view and
        // colour mode stay as they were.
        self.placement = Transform::START;
        self.camera.reset();
        Ok(())
    }
}

/// The window title (§ 19): the file, then the FPS once there's a reading.
fn window_title(name: &str, reading: Option<FpsReading>) -> String {
    match reading {
        Some(r) => format!("scop — {name} — {:.0} fps ({:.1} ms)", r.fps, r.frame_ms),
        None => format!("scop — {name}"),
    }
}

impl App for Scop {
    fn setup(&mut self, gfx: &mut Gfx<'_>) -> Result<(), RunError> {
        let info = gfx.info();
        print_gpu_info(&info);
        // One default pipeline per style (§ 18, § 24).
        for style in STYLES {
            if gfx.supports(style) {
                self.pipelines
                    .push((style, gfx.create_default_pipeline(style)?));
            } else {
                println!("{style:?}: not supported by this GPU, V skips it");
            }
        }
        let (smallest, largest) = info.point_size_range;
        println!("point sizes: {smallest}..{largest} px");
        println!("{CONTROLS}");
        for mesh in &self.model.meshes {
            self.meshes.push(gfx.create_mesh(mesh)?);
        }
        self.model.meshes = Vec::new(); // uploaded
        for picture in &self.pictures {
            self.textures.push(gfx.create_texture(picture)?);
        }
        for colour in PROP_COLOURS {
            self.prop_meshes.push(gfx.create_mesh(&prop_cube(colour))?);
        }
        // § 23: the sky — also the environment chrome reflects.
        if let Some(faces) = self.sky_faces.take() {
            let [a, b, c, d, e, f] = &faces;
            self.sky = Some(gfx.create_default_sky([a, b, c, d, e, f])?);
        }
        Ok(())
    }

    fn update(&mut self, input: &Input, dt: f32, gfx: &mut Gfx<'_>) -> Flow {
        if input.pressed(Key::Escape) {
            return Flow::Exit;
        }
        // § 19: a new reading every half-second; the title shows the latest.
        if let Some(reading) = self.fps.tick(dt) {
            self.fps_reading = Some(reading);
        }
        // § 21: a saved shader → compile → swap. Errors keep the old shaders.
        if let Some(watch) = &mut self.shader_watch
            && watch.poll(dt)
        {
            match watch.compile_all() {
                Ok(compiled) => self.reload_shaders(&compiled, gfx),
                Err(err) => eprintln!("scop: shaders not reloaded:\n{err}"),
            }
        }
        // § 19: files dropped on the window. Only the last .obj stays on screen.
        for path in input.dropped_files() {
            self.open_dropped(path, gfx);
        }
        if input.pressed(Key::E) {
            self.sky_on = !self.sky_on; // § 23
        }
        if input.pressed(Key::C) {
            self.colour_mode = self.colour_mode.next();
            println!("colours: {}", describe(self.colour_mode));
        }
        if input.pressed(Key::M) {
            self.live_reflections = !self.live_reflections; // § 25
            let kind = if self.live_reflections {
                "live"
            } else {
                "fixed sky"
            };
            println!("reflections: {kind}");
        }
        if input.pressed(Key::H) {
            self.show_controls = !self.show_controls; // § 25 extra
        }
        if input.pressed(Key::P) {
            self.props_on = !self.props_on; // § 25 extra
        }
        self.props_time += dt;
        // § 20: drag with the left button to orbit; B switches the light.
        if input.mouse_held(MouseButton::Left) {
            let (dx, dy) = input.mouse_delta();
            self.camera.drag(dx, dy);
        }
        if input.pressed(Key::B) {
            self.light_on = !self.light_on;
        }
        // § 22: G switches to the crowd; the copies keep turning either way.
        if input.pressed(Key::G) {
            self.crowd = !self.crowd;
            let copies = if self.crowd { CROWD_SIDE.pow(3) } else { 1 };
            println!("copies: {copies}");
        }
        self.crowd_time += dt;

        // Toggles: `pressed`, so one press = one switch (Q-13.3).
        if input.pressed(Key::Space) {
            self.spinning = !self.spinning;
        }
        if input.pressed(Key::R) {
            self.placement = Transform::START;
            self.spin = 0.0;
            self.spinning = true;
            self.camera.reset();
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
        let p = &mut self.placement;
        let step = MOVE_SPEED * self.model.radius * dt;
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
        if self.spinning {
            self.spin += SPIN_SPEED * dt;
        }

        // Zoom: the wheel's notches this frame, plus Z / X as notches per second.
        let key_zoom =
            f32::from(u8::from(input.held(Key::Z))) - f32::from(u8::from(input.held(Key::X)));
        self.camera
            .zoom(input.scroll() + key_zoom * ZOOM_KEY_NOTCHES_PER_SECOND * dt);
        Flow::Continue
    }

    fn draw(&self, frame: &mut Frame) {
        frame.clear(BACKGROUND);
        // The engine only applies the title when it changes.
        frame.set_title(window_title(&self.model.name, self.fps_reading));

        // Camera, once per frame (the noticeboard): framed on the model's sphere, the
        // zoom only changes the lens (§ 18), standing on its orbit (§ 20).
        let radius = self.model.radius;
        let framing = self.camera.framing(radius);
        let eye = self.camera.eye(framing.distance);
        let view = self.camera.view(framing.distance);
        frame.set_camera(self.camera.projection(frame.aspect(), framing) * view);
        frame.set_camera_position(eye); // § 23: chrome's reflections start from here
        frame.set_light(LIGHT_DIRECTION, if self.light_on { 1.0 } else { 0.0 });
        // § 25: live chrome — capture what's around the model, from its centre. The
        // model itself is left out of the probe (below), or it would fill its own cube.
        let live = self.live_reflections && self.colour_mode == ColourMode::Chrome;
        if live {
            frame.set_reflection_probe(self.placement.position, framing.near, framing.far);
        }

        // § 23: the sky first; depth writing off, so the model always goes on top.
        if let (true, Some(sky)) = (self.sky_on, self.sky) {
            sky.draw(frame, eye, framing.far);
        }

        // The model's placement (the sticky note): one for the whole file, so the
        // objects keep their places relative to each other. The spin goes on top.
        let mut placed = self.placement;
        placed.angles.y += self.spin;
        let model = placed.matrix(self.model.centre);
        let Some(&(_, pipeline)) = self.pipelines.get(self.style) else {
            self.draw_controls(frame);
            return;
        };
        let params = DefaultParams {
            blend: self.fade.value(),
            colour_mode: self.colour_mode,
            depth_range: engine::depth_range((self.placement.position - eye).length(), radius),
        }
        .params();
        // § 22: the crowd's copies, the same for every object of the model.
        let copies = if self.crowd {
            crowd_copies(self.model.centre, radius, self.crowd_time)
        } else {
            Vec::new()
        };
        // One draw per object, each with its own picture (§ 18, Q-18.3) — with the
        // crowd on, one instanced draw per object; the first object stores the copies,
        // the others reuse its seats (shared range).
        // § 25 extra: the props — ordinary draws, so they appear in the probe's views.
        if self.props_on {
            let places = prop_places(self.placement.position, radius, self.props_time);
            for (&mesh, place) in self.prop_meshes.iter().zip(places) {
                frame.draw(pipeline, mesh, place);
            }
        }
        let mut shared: Option<InstanceRange> = None;
        frame.set_reflective(live); // the model: never in its own reflection
        for (i, &mesh) in self.meshes.iter().enumerate() {
            let Some(texture) = picture_for(i, self.picture_shift, &self.textures) else {
                continue;
            };
            match (self.crowd, shared) {
                (false, _) => frame.draw_with(pipeline, mesh, texture, model, params),
                (true, Some(range)) => {
                    frame.draw_instances_again(pipeline, mesh, texture, model, params, range)
                }
                (true, None) => {
                    shared = frame.draw_instances(pipeline, mesh, texture, model, params, &copies)
                }
            }
        }
        frame.set_reflective(false);
        // § 25 extra: last, so nothing drawn after can paint over it.
        self.draw_controls(frame);
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

    // The same loader a dropped file uses (§ 19).
    let model = match load_model(Path::new(path)) {
        Ok(model) => model,
        Err(err) => {
            eprintln!("scop: {err}");
            return ExitCode::FAILURE;
        }
    };

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

    // § 23: the sky — six faces from resources/sky/ if they're all there, else
    // generated, with the sun where the § 20 light comes from.
    let sky_faces = match engine::load_cube_faces(Path::new("resources/sky")) {
        Ok(Some(faces)) => {
            println!("sky: resources/sky/");
            faces
        }
        Ok(None) => {
            println!("sky: generated (sun where the light comes from)");
            engine::generate_sky(-LIGHT_DIRECTION)
        }
        Err(err) => {
            eprintln!("scop: {err}");
            return ExitCode::FAILURE;
        }
    };

    // § 21: watch the engine's default shaders (their path is written in when the
    // engine is built), if they're still there and glslc is installed.
    let shader_files = engine::default_shader_files();
    let shader_watch = match ShaderWatch::start(&shader_files) {
        Ok(watch) => {
            let folder = shader_files[0].parent().unwrap_or(Path::new("."));
            println!("hot reload: watching {}", folder.display());
            Some(watch)
        }
        Err(why) => {
            println!("hot reload off: {why}");
            None
        }
    };

    let config = WindowConfig {
        title: window_title(&model.name, None),
        width: 1280,
        height: 720,
    };
    let scop = Scop {
        model,
        placement: Transform::START,
        spin: 0.0,
        spinning: true,
        fade: Tween::new(FADE_SECONDS, Easing::Smooth),
        camera: OrbitCamera::new(CameraSettings::default()),
        pictures,
        picture_shift: 0,
        pipelines: Vec::new(),
        style: 0,
        meshes: Vec::new(),
        textures: Vec::new(),
        fps: FpsCounter::new(FPS_WINDOW),
        fps_reading: None,
        colour_mode: ColourMode::Normal,
        light_on: false,
        shader_watch,
        crowd: false,
        crowd_time: 0.0,
        sky_faces: Some(sky_faces),
        sky: None,
        sky_on: false,
        live_reflections: true,
        show_controls: true,
        props_on: false,
        prop_meshes: Vec::new(),
        props_time: 0.0,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn props_ring_the_model_without_touching_it() {
        let middle = Vec3::new(1.0, 2.0, 3.0);
        let places = prop_places(middle, 2.0, 0.7);
        assert_eq!(places.len(), PROP_COLOURS.len());
        for place in places {
            let centre = place.to_cols_array()[3];
            let gap = (Vec3::new(centre[0], centre[1], centre[2]) - middle).length();
            // Outside the model's sphere (+ the cube's own corner), inside the framing.
            assert!(gap - PROP_SIZE * 2.0 * 3f32.sqrt() > 2.0, "{gap}");
            assert!(gap < 2.0 * 2.0, "{gap}");
        }
        let cube = prop_cube([1.0, 0.0, 0.0]);
        assert_eq!((cube.vertices.len(), cube.indices.len()), (24, 36));
    }

    #[test]
    fn controls_are_plain_ascii_for_the_built_in_font() {
        assert!(
            CONTROLS.is_ascii(),
            "the font has no glyph for non-ASCII: use ->, <->"
        );
    }

    #[test]
    fn greys_are_subtle_and_neighbours_differ() {
        for i in 0..1000 {
            let g = face_grey(i)[0];
            assert!((0.25..0.75).contains(&g), "face {i}: {g}");
            assert!(
                (g - face_grey(i + 1)[0]).abs() > 0.05,
                "faces {i} and {}",
                i + 1
            );
        }
    }

    #[test]
    fn scops_loader_gives_grey_faces() {
        // § 24: the engine builds, scop's callback colours: same face → same grey.
        let model = load_model(Path::new("../resources/42.obj")).unwrap();
        let corners = &model.meshes[0].vertices;
        assert!(
            corners
                .chunks_exact(3)
                .all(|t| t.iter().all(|v| v.colour == t[0].colour))
        );
        assert_eq!(corners[0].colour, face_grey(0));
        assert!(load_model(Path::new("../resources/test/broken.obj")).is_err());
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
    fn title_shows_the_file_then_the_fps() {
        assert_eq!(window_title("42.obj", None), "scop — 42.obj");
        let reading = FpsReading {
            fps: 119.6,
            frame_ms: 8.36,
        };
        assert_eq!(
            window_title("42.obj", Some(reading)),
            "scop — 42.obj — 120 fps (8.4 ms)"
        );
    }

    #[test]
    fn the_crowd_fits_the_model_without_overlapping() {
        let (centre, radius) = (Vec3::new(3.0, 1.0, -2.0), 2.0);
        let copies = crowd_copies(centre, radius, 1.5);
        assert_eq!(copies.len(), 512);
        let copy_radius = radius * CROWD_COPY_SIZE / (3f32.sqrt() * CROWD_SIDE as f32);
        let middles: Vec<Vec3> = copies
            .iter()
            .map(|m| (*m * engine::Vec4::point(centre)).xyz())
            .collect();
        for &p in &middles {
            assert!((p - centre).length() + copy_radius <= radius * 1.0001);
        }
        for (i, &a) in middles.iter().enumerate() {
            for &b in &middles[i + 1..] {
                assert!((a - b).length() >= 2.0 * copy_radius, "copies overlap");
            }
        }
    }

    #[test]
    fn every_copy_turns_its_own_way() {
        let copies = crowd_copies(Vec3::ZERO, 1.0, 0.7);
        let cards: Vec<_> = copies.iter().map(|m| m.to_cols_array()).collect();
        for (i, a) in cards.iter().enumerate() {
            assert!(
                cards[i + 1..].iter().all(|b| b != a),
                "two identical copies"
            );
        }
        let later = crowd_copies(Vec3::ZERO, 1.0, 0.8);
        assert!(
            copies
                .iter()
                .zip(&later)
                .all(|(a, b)| a.to_cols_array() != b.to_cols_array())
        );
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
    fn the_spin_goes_on_top_of_the_keys_turn() {
        // § 24: scop's spin is added to the engine Transform's y angle when drawing.
        let mut placed = Transform::START;
        placed.angles.y = 0.5;
        let mut with_spin = placed;
        with_spin.angles.y += 0.25;
        let expected = Transform {
            angles: Vec3::new(0.0, 0.75, 0.0),
            ..Transform::START
        };
        assert_eq!(with_spin.matrix(Vec3::ZERO), expected.matrix(Vec3::ZERO));
    }
}
