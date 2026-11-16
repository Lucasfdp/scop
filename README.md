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
shaders committed in `scop/shaders/spv/`; `make sdk` downloads the compiler into your
home folder if you want it.

### Run

```sh
make run                              # the 42 logo
make run OBJ=resources/teapot.obj     # any .obj file
make run TEX=path/to/picture.tga      # a different picture, no rebuild needed
./target/release/scop path/to/model.obj [path/to/picture.tga]
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
| Space | pause / resume the automatic spin |
| R | reset position and rotation |
| Esc | quit |

Movement follows the screen's axes whatever way the model is turned; every rotation is
around the model's own centre.

### Other targets

```sh
make test     # unit tests (parser, maths, image loader, …)
make spv      # refresh the committed SPIR-V after changing a shader (needs glslc)
make clean / make fclean / make re
```

The default picture is `resources/texture.tga`, embedded in the binary so scop runs from
any directory; replace it and rebuild to change the default. To try another picture
without rebuilding, pass it as the second argument. Pictures must be 24- or 32-bit TGA
(uncompressed or run-length encoded); on macOS:
`sips -Z 1024 -s format tga picture.jpg --out picture.tga`.

## Features and technical choices

- `.obj`: `v` and `f` lines; faces with any number of corners (fan-triangulated),
  `v/vt/vn` tokens, negative indices; anything else is skipped. Broken files give an error
  with the line number.
- One flat grey per `.obj` face (each triangle gets its own corners).
- Perspective camera that frames any model from its bounding box; the model turns around
  the bounding box's centre.
- Own TGA loader (uncompressed and run-length encoded, 24/32-bit); planar texture
  mapping; sRGB-correct colours; the grey ↔ picture fade eased with smoothstep and
  reversible at any moment.
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

### AI usage

<!-- Lucas: this is a DRAFT. Edit until every sentence is true in your own words.
     Fill in the two <…> markers. -->

I used **Claude** (Anthropic) throughout this project, in a way I chose deliberately and
recorded in [`DECISIONS.md`](DECISIONS.md) (2026-11-03).

- **Learning material.** Claude wrote the staged learning plan in `learn/`: 19 sections
  (Rust, how a GPU draws, Vulkan setup, synchronisation, shaders, buffers, 3D maths,
  descriptors, depth, `.obj`, textures, …), each with explanations, a glossary and
  questions.
- **Code.** For each section I first read the material and answered its questions
  (a "gate", graded honestly — misses were re-explained and re-tested). Only then did
  Claude write that section's checkpoint code and walk me through it part by part; the
  section only counted as done once I could explain the code back. Written this way:
  almost all of `engine/` and `scop/` — Vulkan setup, swapchain, frame loop and
  synchronisation, pipeline and shader loading, buffers, the maths library,
  uniforms and descriptors, depth buffer, the `.obj` parser, greys and controls, the TGA
  loader and textures, the fade, window resizing — plus the unit tests, the `Makefile`
  and `scripts/deps.sh`.
- **Mine.** The choices (Rust + Vulkan, the engine/app split, and each decision in
  `DECISIONS.md`, picked from the trade-offs Claude laid out), running and checking every
  checkpoint on my machine and reporting what I saw, catching problems such as the
  `+`/`−` keys not working on a Spanish keyboard, the texture picture <where it comes
  from>, and <anything else you did>.
- **Learning record.** `learn/PROGRESS.md` logs every gate and explain-back, including what I
  got wrong first.

## Learning

This repo contains a staged learning plan: start at [`learn/00-PLAN.md`](learn/00-PLAN.md)
(Spanish: [`learn_es/00-PLAN.md`](learn_es/00-PLAN.md)).
Design decisions are logged in [`DECISIONS.md`](DECISIONS.md).
