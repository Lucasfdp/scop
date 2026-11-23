*This project has been created as part of the 42 curriculum by luferna3.*

# scop

## Description

**scop** is a small 3D viewer: it loads a `.obj` model (the 42 logo, a teapot, or any
file you give it) and shows it spinning in perspective, with every face in its own
subtle shade of grey. Press a key and a picture fades smoothly onto the model; press it
again and it fades back.

It's an introduction to GPU rendering, written in **Rust** on **Vulkan** (through MoltenVK
on macOS). As the subject requires, everything except window and event handling is
written by hand: the `.obj` parser, the matrix library, shader loading, and — by choice —
the image loader.

The code is split in two:

- **`engine/`** — a reusable rendering library: Vulkan setup, swapchain and frame loop,
  buffers and textures, depth buffer, maths, `.obj` and `.tga` loaders. Meant to grow into
  the base for later projects (ft_newton, ft_vox, …).
- **`scop/`** — the viewer itself: which model to show, the controls, the greys and the fade.

## Instructions

### Build

```sh
make
```

One line: checks for Rust, the shader compiler (`glslc`) and Vulkan, installs what it can
without admin rights (Rust via rustup; on macOS `glslc` and MoltenVK via Homebrew), then
builds in release mode. If `glslc` isn't available, the build uses the pre-compiled
shaders committed in `engine/shaders/spv/`; `make sdk` downloads the compiler into your
home folder if you want it.

### Run

```sh
make run                              # the 42 logo
make run OBJ=resources/teapot.obj     # any .obj file
make run TEXTURE=path/to/picture.tga  # a different picture, no rebuild needed
make run OBJ=resources/test/concave.obj TEXTURE="a.tga b.tga"  # one picture per object
./target/release/scop path/to/model.obj [picture.tga ...]
```

### Controls

| Key | Action |
|---|---|
| ← → ↑ ↓ | move left / right / up / down |
| W / S | move closer / further |
| I / K | turn around x (tilt) |
| J / L | turn around y (spin) |
| U / O | turn around z (roll) |
| T | fade between the grey faces and the picture |
| N | rotate which object gets which picture |
| V | solid → wireframe → points |
| C | colours: normal → facing (blue front, red back) → depth (near white, far dark) → chrome (reflects the sky) |
| E | sky on / off |
| mouse wheel, Z / X | zoom in / out (narrows the field of view; the camera stays put) |
| mouse drag | orbit the camera round the model |
| B | light on / off (one sun-like light, fixed in the world) |
| G | one model ↔ a crowd of 512 copies, each spinning its own way (instancing: one draw per object) |
| Space | pause / resume the automatic spin |
| R | reset position, rotation, zoom and camera |
| Esc | quit |

Drop a `.obj` file on the window to open it; a broken file is reported in the terminal
and the current model stays. The title shows the file and the frame rate (capped at the
screen's refresh rate: scop waits for vsync).

Movement follows the screen's axes whatever way the model is turned; every rotation is
around the model's own centre.

### Other targets

```sh
make test     # unit tests (parser, maths, image loader, …)
make spv      # refresh the committed SPIR-V after changing a shader (needs glslc)
make clean / make fclean / make re
```

The default picture is `resources/texture.tga`, embedded in the binary so scop runs from
any directory; replace it and rebuild to change the default. To try other pictures
without rebuilding, pass them after the model: each object in the file (`o` / `g` lines)
takes the next picture in turn. Pictures must be 24- or 32-bit TGA
(uncompressed or run-length encoded); on macOS:
`sips -Z 1024 -s format tga picture.jpg --out picture.tga`.

## Features and technical choices

- `.obj`: `v` and `f` lines; faces with any number of corners (fan-triangulated),
  `v/vt/vn` tokens, negative indices; anything else is skipped. Broken files give an error
  with the line number.
- Concave and non-flat faces split by ear clipping (bonus); scop prints each model's
  open edges (gaps in the surface). The teapot's lid gap is in the model itself: both
  teapot files report 160 open edges, and macOS Quick Look shows the same gap.
- One flat grey per `.obj` face (each triangle gets its own corners).
- Perspective camera that frames any model from its bounding box; the model turns around
  the bounding box's centre.
- Own TGA loader (uncompressed and run-length encoded, 24/32-bit); sRGB-correct colours;
  box texture mapping (bonus): each face takes the front, side or top projection it faces
  most, with one scale across and down, so no face is streaked or stretched;
  a picture per object (`o` / `g` lines), rotated with N; the grey ↔ picture fade eased with smoothstep and
  reversible at any moment.
- Solid, wireframe and point views; zoom by field of view.
- Frame rate in the title, drag & drop to open models, debug colours (which side of each
  face is showing, and distance from the camera).
- Mouse orbit camera; one directional light with flat shading (off until B, so the
  mandatory greys are unchanged).
- Instancing: G draws 512 copies of the model with one draw call per object.
- Cube-map sky (E), generated at start with the sun where the light comes from (or six
  `.tga` faces from `resources/sky/`: `px nx py ny pz nz`), and a chrome mode (C) that
  reflects it.
- Shader hot reload: save one of the engine's default shaders (`engine/shaders/default.vert`,
  `default.frag`, `sky.vert`, `sky.frag`) while scop runs and the change appears without
  restarting; a broken shader prints `glslc`'s error and the old one keeps running. Needs
  `glslc` and the repo's `engine/shaders/` folder; otherwise it switches off.
  (Run `make spv` afterwards to refresh the committed fallback.)
- Depth buffer, window resizing, validation-layer-clean in debug builds.
- Every real decision — what was chosen, what was rejected and why — is in
  [`DECISIONS.md`](DECISIONS.md).

## Resources

- [Vulkan specification](https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html) — the reference for every call
- [Vulkan Tutorial](https://vulkan-tutorial.com) — the classic step-by-step introduction
- [Khronos Vulkan Guide](https://github.com/KhronosGroup/Vulkan-Guide) — synchronisation, dynamic rendering, best practices
- [`ash` documentation](https://docs.rs/ash) — the Rust Vulkan bindings used
- [`winit` documentation](https://docs.rs/winit) — window and events
- [shaderc / `glslc`](https://github.com/google/shaderc) — GLSL → SPIR-V compiler
- [LearnOpenGL — Coordinate Systems](https://learnopengl.com/Getting-started/Coordinate-Systems) — model / view / projection explained
- [Paul Bourke — OBJ format](https://paulbourke.net/dataformats/obj/) and [TGA format](https://paulbourke.net/dataformats/tga/)
- [GPU Gems 3, ch. 24 — The Importance of Being Linear](https://developer.nvidia.com/gpugems/gpugems3/part-iv-image-effects/chapter-24-importance-being-linear) — sRGB and gamma
- [The Rust Programming Language](https://doc.rust-lang.org/book/)

### Font

The on-screen controls use X11's **misc-fixed 8×13** bitmap font (font-misc-misc,
"Public domain font. Share and enjoy."), converted once into
`engine/assets/font_8x13.tga` and read with scop's own TGA loader — no font library.

### AI usage

I used **Claude** (Anthropic) throughout this project, in a way I chose deliberately and
recorded in [`DECISIONS.md`](DECISIONS.md) (2026-11-03).

**How I worked.** scop was also an experiment in a way of working: the concepts were new
to me (GPUs, Vulkan, 3D rendering), but I treated writing the code differently from
learning them. Each module of a staged learning plan had concept questions I had to pass
*before* any code was written; then Claude wrote that checkpoint's code, and I had to
answer walkthrough questions on it before the module counted as done. The aim was to
understand every part of the project without typing every line by hand — and I enjoyed
building scop this way.

- **Learning material.** Claude wrote the staged learning plan in `learn/`: 25 sections
  (Rust, how a GPU draws, Vulkan setup, synchronisation, shaders, buffers, 3D maths,
  descriptors, depth, `.obj`, textures, …), each with explanations, a glossary and
  questions.
- **Code.** For each section I first read the material and answered its questions
  (a "gate", graded honestly — misses were re-explained and re-tested). Only then did
  Claude write that section's checkpoint code and walk me through it part by part; the
  section only counted as done once I could explain the code back.
- **Mine.** The direction of the project: what was included, why, and how I wanted it to
  work. Claude laid out the options and their trade-offs; the decisions were mine (they
  are recorded in `DECISIONS.md`). I also ran and checked every checkpoint on my machine
  and reported what I saw, which caught problems such as the `+`/`−` keys not working on
  a Spanish keyboard.
- **Texture picture.** `resources/texture.tga` is reused from an earlier project of mine;
  I generated it with an AI image tool (I tried several at the time and don't remember
  which one made it).
- **Learning record.** `learn/PROGRESS.md` logs every gate and explain-back, including what I
  got wrong first.

## Learning

This repo contains a staged learning plan: start at [`learn/00-PLAN.md`](learn/00-PLAN.md)
(Spanish: [`learn_es/00-PLAN.md`](learn_es/00-PLAN.md)).
Design decisions are logged in [`DECISIONS.md`](DECISIONS.md).
