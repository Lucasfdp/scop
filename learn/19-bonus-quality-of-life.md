---
section: 19
slug: bonus-quality-of-life
tier: T1
status: done
prereqs: [16, 18]
terms: [frames per second, frame time, vsync cap, averaging window, title throttling, dropped file, runtime mesh replacement, stale handle, debug view, gl_FrontFacing, winding order, inside-out face, linear depth, push params]
---

# Section 19 — Quality of life: FPS, drag & drop, debug colours (bonus)

**Depth: T1** — mostly reuse: § 5's frames in flight, § 16's `device_wait_idle`, § 9's `w`,
§ 15's push block. Bonus extras (owner's picks, 2026-11-18): **FPS in the title**, **drop a
`.obj` on the window to load it**, **debug colour modes (facing / depth)**.
**You'll be able to:** see how fast scop really runs, swap models without restarting, and
look *inside* the picture the GPU makes: which way each face points and how far away it is.

---

## The idea

**FPS.** Frames per second: how many pictures scop finishes each second. The engine already
knows `dt` (seconds since the last frame), so `1 ÷ dt` is "the FPS right now" — but it jumps
around every frame and is unreadable. Instead, count frames for **half a second**, then show
`frames ÷ time`. The number changes twice a second and means something.

There's a ceiling you'll hit straight away: scop presents with **FIFO** (§ 4), which waits
for the screen's refresh. On a 60 Hz screen scop can't show more than 60 frames a second, on
a 120 Hz MacBook 120 — however small the model. Hitting the ceiling means "at least as fast
as the screen", not "this is the limit of the GPU". **Frame time** (milliseconds per frame) is
the same number turned round, and the one programmers compare: 60 fps = 16.7 ms.

**Drag & drop.** The window system tells the app "a file was dropped on you" — one more
event, like a key. scop reads and parses it the same way as the command-line model. If it's
broken, scop prints the error and **keeps the old model**: a bad drop never closes the viewer.

Swapping models mid-run is the new problem. Until now meshes were made once, in `setup`.
Now they're made and **destroyed** while frames are in flight — and the GPU may still be
drawing the old model in a frame submitted a moment ago (§ 5: up to two frames). Destroying
its buffers then is a use-after-free on the GPU. scop's fix is the one you already know:
`device_wait_idle` (§ 16) before destroying. A one-frame pause, only when a file is dropped.

**Debug colours.** Replace the picture with colours that *mean* something:

- **Facing:** paint the side of each triangle you're seeing — front = blue, back = red. A
  model whose faces point the wrong way ("inside out") shows red on its outside.
- **Depth:** paint each pixel by how far it is from the camera — near = white, far = dark.
  You see the model's shape in depth, the same thing the depth buffer (§ 11) compares.

C cycles: normal → facing → depth.

**Where the analogy stops being true:** "front" isn't something a triangle *has*. The GPU
decides it from the order the three corners land on screen (clockwise or anticlockwise) —
so anything that mirrors the picture (like the y flip in § 9's perspective card) swaps front
and back unless the pipeline's `front_face` setting accounts for it.

---

## The names for it

**frames per second (FPS)** — frames finished per second, averaged over a short window.

**frame time** — milliseconds per frame: `1000 ÷ FPS`. Adds up honestly (two 8 ms jobs = 16 ms);
FPS doesn't.

**vsync cap** — FIFO presentation waits for the screen's refresh, so FPS can't exceed the
refresh rate (60, 120 …). The other present modes (MAILBOX, IMMEDIATE) lift the cap.

**averaging window** — how long frames are counted before the number is updated (0.5 s here).

**title throttling** — changing the window title only when its text changes. Setting it every
frame is slow on some systems (macOS redraws the title bar each time).

**dropped file** — winit's `WindowEvent::DroppedFile(path)`: one event per file dropped.

**runtime mesh replacement** — creating and destroying meshes after `setup`, while frames are in
flight. Needs `Gfx` during `update`, and a wait before destroying.

**stale handle** — a `MeshId` whose mesh was destroyed. Drawing with it must be an error the
engine reports, never a GPU crash: the slot is emptied, not reused.

**debug view** — false colours that show a hidden number (facing, depth, normals, UVs…).

**gl_FrontFacing** — fragment shader input: `true` if the triangle is seen from its front.
Always `true` for points and lines (they have no sides).

**winding order** — whether a triangle's corners go clockwise or anticlockwise *on screen*. The
pipeline's `front_face` says which one counts as front (scop: `COUNTER_CLOCKWISE`).

**inside-out face** — a face whose corners are listed clockwise when seen from outside: its front
points into the model.

**linear depth** — the distance from the camera along the view direction. After the perspective
card it's simply `w` (§ 9: "the last row copies the distance into w").

**push params** — four app-defined numbers in the push block (§ 15), next to the model matrix:
scop puts the fade, the debug mode and the depth range there.

---

## How it actually works

### 1. FPS in the title (engine + scop)

```
engine: FpsCounter { frames, elapsed }      // a small public helper
        tick(dt): frames += 1, elapsed += dt
                  elapsed ≥ 0.5 → report (frames / elapsed, elapsed / frames × 1000), reset
        Frame::set_title(text)              // the app asks for a title
        platform: window.set_title only if text ≠ last text   (title throttling)
scop:   title = "scop — 42.obj — 120 fps (8.3 ms)"
```

### 2. Drag & drop (engine + scop)

```
engine: WindowEvent::DroppedFile(path) → Input::dropped_files() for this frame
        App::update now also gets  gfx: &mut Gfx   (create meshes mid-run)
        Gfx::destroy_mesh(id): device_wait_idle, destroy buffers, empty the slot
        meshes: Vec<Option<Mesh>>  → drawing a destroyed id = GpuError::UnknownHandle
scop:   dropped .obj → read → parse → (error? print, keep old model)
                     → new object meshes → destroy the old ones → new centre, radius
                     → reset placement and zoom; pictures and view mode stay
        anything else dropped → "scop: x.png: drop a .obj file"
```

### 3. Debug colours (engine push block + scop shaders)

```
push block: mat4 model | vec4 params          // 64 + 16 = 80 bytes (≤ 128, always allowed)
            params = (blend, mode, depth_near, depth_far)   — scop's meaning, not the engine's
engine:     Frame::draw_with(pipeline, mesh, texture, model, params: [f32; 4])
            draw_textured = draw_with(…, [blend, 0, 0, 0])   (unchanged for callers)
mesh.vert:  out distance = gl_Position.w                    // linear depth (§ 9)
mesh.frag:  mode 0: mix(grey, picture, blend)              // as before
            mode 1: gl_FrontFacing ? blue : red
            mode 2: t = clamp((distance − near) / (far − near)); grey = 1 − t
scop:       depth range = the model's own: camera distance − position.z ± radius
            C: mode = (mode + 1) % 3
```

Depth mode uses `w`, **not** `gl_FragCoord.z` (the stored depth). Stored depth is squeezed
towards 1 by the perspective card: with scop's tiny near plane, the whole model sits at about
0.99–1.0 — a white blob (Q-19.5). Distance spread over the model's own range uses the full
white-to-dark scale.

### Decisions

| Choice | Over | Why |
|---|---|---|
| **FPS averaged over 0.5 s**, plus frame time | `1 ÷ dt` every frame | Readable; frame time is what you compare |
| **`FpsCounter` in the engine, title chosen by the app** | Engine writes the title itself | Every app gets the counter; each decides what its title says |
| **`update` gets `&mut Gfx`** | A separate "files dropped" callback | ft_vox (chunks) and ft_newton (spawned objects) will need meshes mid-run too |
| **`device_wait_idle` before destroying** | A deletion queue (destroy after `FRAMES_IN_FLIGHT` frames) | Drops are rare; one pause is invisible. The queue is for ft_vox, which swaps meshes every frame |
| **`Vec<Option<Mesh>>`, ids never reused** | Reusing freed slots | A stale id is an error, never someone else's mesh |
| **vec4 push params, meaning set by the app** | Engine fields named `debug_mode`, `depth_range` | Keeps scop's ideas out of the engine (CLAUDE.md rule 6) |
| **Depth from `w`**, range from the model's own size | `gl_FragCoord.z` | Stored depth is non-linear and crowds near 1 |
| **Facing: blue front, red back** | Normals as colours | Answers the one question that matters for `.obj` files: inside out or not |

---

## Known traps

- **FPS stuck at exactly 60 / 120** — the vsync cap, not a slow program (Q-19.1). Compare frame
  times in a *release* build: debug builds and validation layers are much slower.
- **Title flickers or the app stutters** — setting the title every frame; only set it when the
  text changes.
- **Crash or validation error right after a drop** — old meshes destroyed while a frame in flight
  still uses them (Q-19.3). Wait first.
- **Dropped model invisible or tiny** — centre and radius still the old model's; recompute both.
- **Broken drop closes scop** — an error returned from `update` instead of printed. A bad file is
  the user's mistake, not a reason to exit.
- **Validation error "push constant range"** — push block size changed in the shaders but not in
  the pipeline layout (or the reverse): 68 → 80 bytes everywhere at once (§ 15, Q-15.4).
- **Everything red in facing mode** — either the file is inside out, or front/back is swapped by the
  y flip (Q-19.4). Test with a model you trust before blaming the file.
- **Points all blue in facing mode** — expected: a point has no sides, so it always counts as front.
  (Wireframe still shows both colours: its lines come from triangles, which do have sides.)
- **Depth mode all white** — `gl_FragCoord.z` used, or the range not following W/S movement.

---

## Checkpoint

**Satisfies (subject):** bonus extras (FPS, drag & drop, debug colours). Method: gate first, then
Claude writes it and walks you through it.

1. **Engine:** `FpsCounter`; `Frame::set_title` with throttling; `Input::dropped_files()`;
   `App::update(…, gfx)`; `Gfx::destroy_mesh` (wait, destroy, empty slot); push block
   `model + vec4 params`, `Frame::draw_with`.
2. **scop:** title `scop — <file> — <fps> fps (<ms> ms)`; drop a `.obj` → load or report;
   C cycles normal / facing / depth; `distance` from the vertex shader; depth range from the
   model; controls list and README updated.
3. **Tests:** `FpsCounter` averages and resets; title only changes when the text does; a stale
   `MeshId` is an error; push bytes are 80 with params after the matrix; broken drop keeps the
   old model (load function returns an error, state unchanged).

**Verify:** run a *release* build — the title shows FPS at your screen's refresh rate; drop
`teapot.obj`, then `broken.obj` (error printed, teapot stays), then `concave.obj`; C on the logo:
facing should be all blue from outside (if not, find out which cause — Q-19.4), depth white at
the front fading to dark; validation silent throughout.

---

## Questions

Q-19.1 (recall) The title says exactly 120 fps whether the model is the 42 logo or the teapot.
What is limiting it, and what does that number tell you — and not tell you?

Q-19.2 (application) Why average the FPS over half a second instead of showing `1 ÷ dt` every
frame? Give two reasons.

Q-19.3 (prediction) You drop a new model, and scop destroys the old model's buffers straight
away, without waiting. What can go wrong, when, and would validation tell you?

Q-19.4 (prediction) In facing mode the 42 logo is red on every outside face. Give two possible
causes, and one quick test that tells them apart.

Q-19.5 (application) Why would colouring by `gl_FragCoord.z` (the stored depth) paint the whole
model almost white, and what does scop use instead?

Answers: `ANSWERS.md` § 19
