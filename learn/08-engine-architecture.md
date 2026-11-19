---
section: 8
slug: engine-architecture
tier: T2
status: done
prereqs: [7]
terms: [public API, crate boundary, coupling, inversion of control, App trait, default method, generic function, newtype, typed handle, draw list, input abstraction, delta time, re-export]
---

# Section 8 — Splitting engine from app

**Depth: T2** — this is your stated second goal: an `engine` crate you can take into
ft_newton, ft_vox and ft_minecraft. A messy boundary now means rewriting the engine for
every project; a clean one means each project only writes what's new. It's also what an
evaluator sees first when they open the repo.
**You'll be able to:** say what belongs in the engine and what belongs in the app, design
a small public API around that, and refactor scop so it uses nothing *but* that API.

---

## The idea

Look at how scop and the engine talk today:

```rust
platform::run(config, MESH_SHADERS, quad(), print_gpu_info, pulsing_blue)
```

Five arguments, each added by one section. And inside, the engine knows things it
shouldn't: it builds exactly **one** pipeline and uploads exactly **one** mesh at
startup, and `draw_frame` draws that one mesh. ft_newton needs dozens of objects moving
every frame; this API can't do that without being rewritten.

Think of the engine as a **theatre company** you hire, and each project as a **play**.

- The company owns the **building and the crew**: the stage (window), the lighting rig
  (GPU setup), the scene changes (swapchain, frames, sync). Every play needs these; no
  play should rebuild them.
- The play owns the **script**: what's on stage, what happens when someone presses a
  key, what colour the backdrop is.
- The company runs the evening. It doesn't wait for the script to say "now refresh the
  lights"; it **calls the actors when it's their turn**: "set up your props", "the
  audience did something", "it's your scene — what's on stage?"

That last point has a name: **inversion of control**. Normally *your* code calls a
library. Here the engine owns the loop and calls *your* code at the right moments —
"don't call us, we'll call you". `winit` already works this way with you (§ 2's
`ApplicationHandler`); now your engine does the same for scop.

The script talks to the company through a short, written agreement: "you can ask for a
prop (a mesh), a lighting setup (a pipeline), and each scene you can say what's on
stage". That agreement is the **public API**. Everything else — swapchains, fences,
memory types — stays backstage, invisible to the play. If the company rebuilds its
rigging (a new `winit` version, render passes → dynamic rendering), plays don't change.

**Where the analogy stops being true:** a theatre company and a play are separate
businesses that can argue about the contract. Here you write both sides — so it's
tempting to "just reach backstage" when the API lacks something. Every time you do,
the next project inherits that shortcut.

---

## The names for it

**public API** — everything a crate marks `pub` and reachable from outside: the types,
functions and traits other crates may use. **API** = **A**pplication **P**rogramming
**I**nterface (§ 2): the agreed surface between two pieces of code.

**crate boundary** — the line between two crates. Rust enforces it: scop can only touch
what the engine marks `pub`. `pub(crate)` (§ 5) means "public inside this crate only" —
backstage.

**coupling** — how much one piece of code depends on the details of another. *Tight*
coupling: scop imports `ash` and `winit` types, so changing the engine's internals breaks
scop. *Loose* coupling: scop only names engine types.

**inversion of control** — the framework owns the main loop and calls your code, instead
of your code calling the framework. Also called "the Hollywood principle".

**App trait** — the contract scop implements: a Rust `trait` (§ 1) with the methods the
engine will call — set up, react to input, describe the frame.

**default method** — a trait method with a body: `fn update(&mut self, …) {}`.
Implementers may override it or leave it. Lets an app implement only what it needs.

**generic function** — `fn run<A: App>(app: A)` (or `fn run(app: impl App)`): one
function that works for any type implementing `App`. The compiler makes a specialised
copy for each type actually used — no runtime cost.

**newtype** — a struct wrapping one value to give it its own type:
`pub struct MeshId(usize)`. A `MeshId` and a `PipelineId` are both numbers inside, but the
compiler won't let you pass one where the other is expected.

**typed handle** — a newtype id the engine gives the app instead of the object itself.
"Here's ticket `MeshId(0)`; the mesh stays in the cloakroom." The engine keeps
ownership, so the app can't destroy a mesh while the GPU is still drawing it.

**draw list** — what the app hands the engine each frame: "clear to this colour; draw
mesh 0 with pipeline 0". A list of *requests*, recorded into the command buffer by the
engine.

**input abstraction** — the engine's own `Key` enum, translated from `winit`'s key codes.
scop says `Key::Space`, never `winit::keyboard::KeyCode::Space`.

**delta time** — seconds since the previous frame, passed to `update`. Movement scaled
by it runs at the same speed at 30 or 120 frames per second (§ 13's rotation).

**re-export** — `pub use crate::mesh::Vertex;` in `lib.rs`: makes an item available at
`engine::Vertex`, so the public API is one tidy list instead of the engine's internal
module layout.

---

## How it actually works

### 1. Sorting what goes where

| Engine (every project) | scop (this project only) |
|---|---|
| Window, event loop, key translation | Which keys do what (§ 13, § 15) |
| Vulkan setup, swapchain, frames, sync | The clear colour |
| Pipelines from SPIR-V, the SPIR-V loader | The GLSL shaders themselves |
| Buffers, uploads, meshes | The `.obj` parser? — **engine** (ft_newton loads models too), § 12 |
| Vertex layout | The model file path, the controls, the texture choice |
| Draw list → command buffer | What to draw each frame |

A test for anything new: *would ft_newton need this, unchanged?* Yes → engine. No → scop.

### 2. The API shape

```rust
// engine/src/lib.rs — the whole public surface, re-exported
pub use app::{App, Flow, Frame, Gfx};
pub use input::{Input, Key};
pub use mesh::{MeshData, Vertex};
pub use pipeline::Shaders;
pub use renderer::{MeshId, PipelineId};
pub use platform::{run, WindowConfig, RunError};
```

```rust
pub trait App {
    /// Once, after the window and GPU exist: create pipelines and meshes.
    fn setup(&mut self, gfx: &mut Gfx) -> Result<(), RunError>;
    /// Every frame, before drawing. Return Flow::Exit to quit.
    fn update(&mut self, _input: &Input, _dt: f32) -> Flow { Flow::Continue }
    /// Every frame: say what to draw.
    fn draw(&self, frame: &mut Frame);
}
```

- **`Gfx`** — the app's window onto the GPU at setup: `create_pipeline(Shaders) ->
  PipelineId`, `create_mesh(&MeshData) -> MeshId`, plus read-only info (GPU name,
  swapchain format) for printing.
- **`Frame`** — the draw list for one frame: `clear(colour)`, `draw(pipeline, mesh)`.
- **`Input`** — which `Key`s are held down and which were pressed this frame.
- **`Flow`** — `Continue` or `Exit`. Esc moves out of the engine into scop: quitting is a
  script decision.

### 3. Inside the engine

`GpuContext` stops owning "the" pipeline and "the" mesh. A `Renderer` (engine-private
fields) owns a `Vec<Pipeline>` and a `Vec<Mesh>`; `PipelineId(i)` and `MeshId(i)` are
indices into them. `draw_frame` takes the frame's draw list and records each request:
bind pipeline, bind buffers, `draw_indexed`. Everything is still destroyed in `Drop`,
after `device_wait_idle` — the app never destroys GPU objects itself.

### 4. Inside scop

```rust
struct Scop { pipeline: Option<PipelineId>, quad: Option<MeshId>, start: Instant }

impl App for Scop {
    fn setup(&mut self, gfx: &mut Gfx) -> Result<(), RunError> { … create both, print GPU info … }
    fn update(&mut self, input: &Input, _dt: f32) -> Flow {
        if input.pressed(Key::Escape) { Flow::Exit } else { Flow::Continue }
    }
    fn draw(&self, frame: &mut Frame) { frame.clear(pulsing_blue(…)); frame.draw(…); }
}
```

`main` becomes: read args, build `Scop`, `engine::run(config, scop)`. **scop's
`Cargo.toml` no longer needs `ash` or `winit`, and its code names neither.**

### Decision: how scop plugs in

| | **A. `App` trait** (recommended) | B. Struct of callbacks | C. App owns the loop |
|---|---|---|---|
| Shape | `impl App for Scop` | `run(Callbacks { setup: …, draw: … })` | `loop { engine.poll(); engine.draw(); }` |
| App state | Lives in `self`, naturally | Captured in closures or globals — awkward to share between callbacks | Ordinary locals |
| Fits `winit` 0.30 | Yes — same callback style | Yes | Poorly — `winit` 0.30 wants to own the loop |
| Grows to ft_newton | Add a default method; old apps still compile | Add a field; every app must change | Each app re-writes the loop |
| Cost | A trait to learn the shape of | Many function types | Fighting `winit` |

Recommendation: **A**. Confirm or override at the gate.

### 5. What deliberately stays small

No scene graph, no entity system, no asset manager, no hot reload. One window, a few
pipelines, a few meshes. The plan's rule (§ 00): the engine grows when a project needs
it, not before.

---

## Known traps

- **The leaky abstraction** — a `pub` field of type `vk::Buffer` or `winit::KeyCode`
  in the public API. scop now depends on `ash`/`winit` again, just indirectly. Check:
  scop's `Cargo.toml` has only `engine`; `cargo build` would fail on any `ash::` in scop.
- **Ids from the wrong engine or out of range** — `MeshId(7)` when there are 2 meshes.
  Keep the number inside private — `pub struct MeshId(usize)`, not
  `pub struct MeshId(pub usize)` — so only the engine can make one.
- **App holding GPU objects** — if `create_mesh` returned a `Mesh` that scop owns, scop
  could drop it mid-frame: use-after-free on the GPU (§ 5 "in use"). Handles prevent it.
- **Over-engineering** — designing for ft_minecraft's needs now. Build only what scop
  uses; leave clear *places* to grow (default methods, new `Frame` requests).
- **Breaking the drop order** — the refactor moves objects around; `Graphics` must
  still drop the GPU before the window, and meshes/pipelines before the device.
- **Behaviour change hidden in a refactor** — a refactor should look identical on screen.
  Same quad, same pulse, Esc still quits, no validation messages. Check before and after.

---

## Checkpoint

**Satisfies:** no visible change — that's the point. It sets up § 12–15, where scop
adds controls and models without touching engine internals.
(Method: gate first, then Claude writes it and walks you through it.)

1. Engine: `App` trait, `Gfx`, `Frame` (draw list), `Flow`, `Input` + `Key`, `MeshId` /
   `PipelineId` newtypes with private fields, `run(config, app)`.
2. Engine: a renderer owning pipelines and meshes in `Vec`s; `draw_frame` records the
   frame's draw list.
3. Engine: one tidy re-export list in `lib.rs`; internals `pub(crate)` or private.
4. scop: `struct Scop` implementing `App`; Esc handled in scop via `Flow::Exit`.
5. scop's `Cargo.toml` depends only on `engine`; no `ash`, `winit` or `vk::` anywhere
   in scop.
6. **Prove the boundary:** try `use engine::buffer::Buffer;` in scop — it must fail to
   compile (private). Remove it.

**Verify:** identical behaviour to § 7 (pulsing background, four-colour quad, Esc quits,
no validation messages); `cargo test` and `cargo clippy` clean.

---

## Questions

Q-8.1 (recall) What does *inversion of control* mean in this engine: who owns the main
loop, and who calls whom?

Q-8.2 (application) Sort into engine or scop: (a) the path of the `.obj` file to show;
(b) the swapchain; (c) the backdrop colour; (d) the SPIR-V loader; (e) the GLSL source
of scop's shaders; (f) translating `winit` key codes; (g) which key toggles the texture.

Q-8.3 (prediction) Suppose `Gfx::create_mesh` returned the `Mesh` itself (with its
buffers) instead of a `MeshId`, and scop dropped it while frames were in flight. What
happens, and how does a handle prevent it?

Q-8.4 (prediction) `App::update` receives `winit`'s own `KeyCode` instead of the engine's
`Key`. A year later you update `winit` and it renames some keys. What has to change in
ft_newton, ft_vox and scop — and what would have to change with the engine's `Key`?

Q-8.5 (application) Which of these should be `pub` in the engine, and which `pub(crate)`
or private: `Vertex`, `Buffer`, `MeshId`, `find_memory_type`, `App`,
`GpuContext::draw_frame`? One reason each.

Answers: `ANSWERS.md` § 8
