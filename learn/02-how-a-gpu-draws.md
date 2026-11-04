---
section: 2
slug: how-a-gpu-draws
tier: T2
status: not-started
prereqs: [1]
terms: [CPU, GPU, API, Vulkan, MoltenVK, rendering pipeline, vertex, primitive, vertex shader, rasterisation, fragment, fragment shader, framebuffer, swapchain, NDC, GLSL, SPIR-V, command buffer, queue, winit, event loop]
---

# Section 2 — How a GPU draws a triangle

**Depth: T2** — every Vulkan object in sections 3–11 exists to configure one stage
of the pipeline described here. If you can draw this pipeline from memory, Vulkan
stops being a list of 40 mysterious objects and becomes "the knobs for stage X".
**You'll be able to:** explain, without notes, what happens between "here are three
points" and "a triangle is on screen", and why Vulkan makes you set up so much.
You'll also have a window open — the thing Vulkan will draw into.

---

## The idea

With mlx you drew by writing pixels into an image yourself, on the processor: a loop
over every pixel, deciding its colour. It works, but the processor does each pixel
one after another. A 1920×1080 window is two million pixels, sixty times a second.

A graphics card works differently. It's like a factory floor with thousands of
workers who are individually slow and not very clever, but who all do *the same
small job at the same time*. You don't tell it "colour pixel 1, then pixel 2". You
tell it: "here is the *recipe* for colouring a pixel; here's the data; go". Every
worker runs the recipe on its own pixel in parallel.

Drawing a 3D object is a production line with fixed stations:

1. **Points in.** You give a list of corner points — for a cube, its eight
   corners, each as `(x, y, z)` plus extras like a colour.
2. **Place each point.** A small program you write runs *once per point* and
   answers one question: "where on the screen does this point land?" This is where
   rotation and perspective happen.
3. **Join the dots.** The points are grouped in threes into triangles. Everything
   on a GPU is made of triangles, because three points always lie on a flat surface.
4. **Find the covered pixels.** Fixed hardware works out which pixels each
   triangle covers. You don't write this part.
5. **Colour each pixel.** A second small program you write runs *once per covered
   pixel* and answers: "what colour is this pixel?" Flat grey, or a lookup into a
   kitten picture.
6. **Keep the nearest.** If two triangles cover the same pixel, keep the one
   closest to the camera (§ 11).
7. **Show it.** The finished image is handed to the screen, while the next one is
   already being drawn into a second image.

Libraries like raylib hide this production line behind `DrawModel()`. Vulkan hands
you the whole floor plan: you configure every station yourself, up front.

**Where the factory analogy stops being true:** factory stations run one after the
other on one item. On a GPU, all stations run at the same time on *different*
triangles and frames, and the processor that gave the orders has already moved on
to other work. That second fact is the root of § 5.

---

## The names for it

**CPU** — **C**entral **P**rocessing **U**nit. The main processor: a few very fast,
very clever cores. Runs your Rust program.

**GPU** — **G**raphics **P**rocessing **U**nit. Thousands of simple cores running
the same program on different data. On your Mac, the GPU is part of the same chip
as the CPU (Apple Silicon), but it is still a separate processor you send work to.

**API** — **A**pplication **P**rogramming **I**nterface. The set of functions a
library or system offers you. Vulkan is an API: a list of C functions that the GPU
driver implements.

**Vulkan** — the explicit, low-level graphics API from the Khronos Group (who also
maintain OpenGL), released 2016. Named after Vulcan, the Roman god of fire and
the forge; it began life as AMD's "Mantle" API. "Explicit" means the driver
guesses nothing: you state every setting yourself.

**MoltenVK** — a translation layer that implements Vulkan on top of Apple's Metal
API, since macOS has no native Vulkan driver. "Molten" + "VK" (Vulkan's usual
abbreviation): Vulkan, melted down into Metal. It's why `vkcube` worked.

**rendering pipeline** — the production line above: the fixed order of stages that
turns vertices into pixels. "Pipeline" because each stage feeds the next, and all
stages work simultaneously on different pieces.

**vertex** (plural *vertices*) — one corner point plus whatever data you attach to
it (colour, texture coordinates). Latin for "turning point" or "peak".

**primitive** — the basic shape the vertices are assembled into. For you, always a
triangle. Called "primitive" because it's the smallest drawable unit.

**vertex shader** — the small program that runs once per vertex (stage 2). Its main
output is the vertex's position on screen.

**shader** in general — a program that runs on the GPU. The name is historical: the
first ones only computed *shading* (light and dark). Today they compute anything.

**rasterisation** — stage 4: turning a triangle (a mathematical shape) into the grid
of pixels it covers. From *raster*, the grid of lines an old CRT screen drew; itself
from Latin *rastrum*, "rake".

**fragment** — a candidate pixel produced by rasterisation: "this triangle covers
this pixel, here is the interpolated data". It becomes a real pixel only if it
survives the depth test. That's why the name is *fragment* and not *pixel*.

**fragment shader** — the program that runs once per fragment (stage 5) and outputs
its colour. (OpenGL and Vulkan say "fragment shader"; DirectX calls it a "pixel shader".)

**framebuffer** — the image(s) being drawn into: a colour image, and later a depth
image. A buffer of memory holding one frame.

**swapchain** — a small set (usually 2–3) of framebuffer images that take turns:
the screen shows one while the GPU draws into another, then they swap. A *chain*
of images you *swap* through.

**NDC** — **N**ormalised **D**evice **C**oordinates. The coordinate system the vertex
shader's output ends up in: a box where x and y run from −1 to +1 across the window,
whatever its size in pixels. "Normalised" because the range is fixed; "device"
because it's independent of the window. **In Vulkan, +y points down** (in OpenGL it
points up), and depth runs 0 to 1.

**GLSL** — **O**pen**GL** **S**hading **L**anguage, the C-like language you'll write
shaders in. Despite the name, it works for Vulkan too.

**SPIR-V** — **S**tandard **P**ortable **I**ntermediate **R**epresentation, version 5
("V"). A compiled binary format for shaders. Vulkan only accepts SPIR-V, never GLSL
text, so you compile GLSL → SPIR-V with `glslc` before running — like compiling C to
an object file. Loading those bytes into Vulkan is the part you must write yourself.

**command buffer** — a recorded list of instructions for the GPU ("use this
pipeline, use these vertices, draw 36 vertices"). You record it on the CPU, then
submit it. Like writing a shopping list and handing it over, instead of phoning
in each item.

**queue** — where you submit command buffers. The GPU works through its queue at its
own pace, **asynchronously**: the submit call returns immediately, long before the
GPU finishes.

**winit** — **win**dow **init**ialisation. The Rust crate that creates a window and
delivers keyboard, mouse and resize events. Your mlx equivalent.

**event loop** — the loop that waits for things to happen (key pressed, window
resized, close clicked) and calls your code for each. You hand control to it; it
calls you back, exactly like `mlx_loop` with hooks.

---

## How it actually works

### The pipeline, with real names

```
 CPU side (your Rust)                       GPU side
 ───────────────────                        ────────────────────────────────────────
 vertex data ──► vertex buffer ──────────►  [Vertex shader]  runs per vertex
 (from .obj)     (GPU memory, § 7)               │  outputs position in clip space
                                                 ▼
 per-frame data (rotation, time) ─────────► [Primitive assembly]  vertices → triangles
 (uniforms, § 10)                                │
                                                 ▼
                                            [Rasterisation]  triangle → fragments   (fixed)
                                                 │  interpolates vertex outputs
                                                 ▼
 texture image (§ 14) ────────────────────► [Fragment shader]  runs per fragment
                                                 │  outputs a colour
                                                 ▼
                                            [Depth test]  keep nearest (§ 11)        (fixed)
                                                 │
                                                 ▼
                                            swapchain image ──► screen (§ 4)
```

The two boxes you **program** are the vertex shader and the fragment shader (§ 6).
The others are **fixed function**: you configure them with settings (cull back
faces? test depth?), but you don't write their code.

### Interpolation — the quiet magic of rasterisation

Give the three corners of a triangle the colours red, green and blue. The fragment
shader isn't handed "red" or "green" — it's handed a *blend*, weighted by how close
the pixel is to each corner. Any value the vertex shader outputs gets blended across
the triangle this way. That's how texture coordinates reach each pixel (§ 14), and
how you'll get the smooth transition in § 15. For flat per-face colours (§ 13) you'll
need to *switch this off* — keep that in mind.

### Coordinate spaces, in one line each (maths is § 9)

A vertex passes through several coordinate systems before reaching the screen:

1. **Model space** — as written in the `.obj`, relative to the model's own origin.
2. **World space** — after placing/rotating the model in the scene.
3. **View space** — relative to the camera.
4. **Clip space** — after the perspective projection; this is what the vertex
   shader outputs, as four numbers `(x, y, z, w)`.
5. **NDC** — clip space divided by `w`. This division is what makes far things small.
6. **Screen** — NDC stretched to the window's pixel size. Done by fixed hardware.

### Why Vulkan needs so many objects

OpenGL is one big global state machine: you set "current colour = red" and the
driver figures out the rest, often stalling to recompile things behind your back.
Vulkan asks you to declare everything up front so the driver never has to guess.
Here's the full map of objects you'll meet — **don't learn them now**, just see that
each one belongs to a station:

| Object | What it's for | Section |
|---|---|---|
| Instance | Your connection to the Vulkan library | 3 |
| Physical device | One actual GPU on the machine | 3 |
| Logical device + queue | Your session with that GPU, and its inbox | 3 |
| Surface + swapchain | The window's images to draw into | 4 |
| Command pool + buffer | The shopping lists of GPU work | 5 |
| Fence + semaphore | "Wait until the GPU is done" signals | 5 |
| Shader module + pipeline | The two programs + every fixed-stage setting, baked together | 6 |
| Buffer + device memory | Vertex/index data on the GPU | 7 |
| Descriptor set | How shaders find uniforms and textures | 10 |
| Image + image view + sampler | Depth buffer and textures | 11, 14 |

### The window: `winit`

Vulkan does not create windows. `winit` does, and later hands Vulkan a
**window handle** (via the `raw-window-handle` crate) so Vulkan can draw into it.

Since `winit` 0.30, you implement a trait (`ApplicationHandler`) on your own struct,
and the event loop calls its methods: one when the app is ready to create its
window (`resumed`), one for every window event (`window_event` — key presses,
resize, close requested). Read the docs for the **exact version you pin** — the API
has changed between versions, and most online examples are for older ones.

---

## Known traps

- **Thinking the GPU runs your command when you call it.** Submit returns
  immediately; the GPU may still be working on the previous frame. If you change
  data the GPU is still reading, you get flicker or garbage. Fixed in § 5.
- **Expecting Vulkan to accept GLSL.** `vkCreateShaderModule` wants SPIR-V bytes.
  Passing GLSL text gives a validation error or a crash.
- **Upside-down models.** Vulkan's NDC has +y pointing down; most tutorials for
  OpenGL assume +y up. If your model appears flipped, this is why (fixed in the
  projection matrix in § 10).
- **Old `winit` examples.** Code with `EventLoop::run(move |event, ...|` closures is
  the pre-0.30 API. If it doesn't compile, check the version.
- **Window created outside `resumed`.** In recent `winit`, creating the window
  before the event loop is running is deprecated (and fails on some platforms).

---

## Checkpoint

**Satisfies:** "inside a window", and the input-handling half of "rotate and
translate using any user input method".

1. Add `winit` as a dependency of the **engine** crate. Pin a version
   (e.g. `winit = "=0.30.x"` with the current patch).
2. In `engine`, create a `platform` (or `window`) module that owns the event loop and
   the window. The scop binary should not import `winit` directly — it calls the
   engine. (This is the first engine/app boundary decision; log it in
   `DECISIONS.md` if you choose differently.)
3. Open a window titled `scop`, 1280×720.
4. Handle: close button → exit cleanly; `Esc` → exit cleanly; window resized →
   print the new size; any other key → print which key.
5. Keep the § 1 behaviour: the `.obj` path is still read from the arguments first,
   and a bad path exits *before* opening the window.

**Verify:** window opens; resizing prints sizes; `Esc` and the close button both
exit with code 0; `cargo clippy` is clean.

---

## Questions

Q-2.1 (recall) Which two stages of the pipeline do you write programs for, and how
often does each one run?

Q-2.2 (application) A cube has 8 corners, but you'll send 36 vertices to draw it.
Where does 36 come from?

Q-2.3 (prediction) Your vertex shader outputs the three corners of a triangle with
colours red, green and blue, and your fragment shader just outputs the colour it
receives. What does the triangle look like on screen, and which stage is
responsible?

Q-2.4 (prediction) You record a command buffer, submit it, and on the very next line
of Rust overwrite the vertex data it uses. What might you see, and why?

Q-2.5 (prediction) You take a vertex shader from an OpenGL tutorial, unchanged,
and render a model of a standing person. What do you expect to see in Vulkan?

Answers: `ANSWERS.md` § 2
