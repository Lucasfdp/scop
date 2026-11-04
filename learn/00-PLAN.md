# scop — Learning Plan

**Mode:** 42 (outer core)
**Goal:** build a foundation in 3D rendering, plus a reusable engine base
**Started:** 2026-11-02
**Stack:** Rust · Vulkan (`ash`, MoltenVK on macOS) · `winit` · GLSL → SPIR-V (`glslc`)
**Languages:** learn/ (EN) · learn_es/ (ES)

## What this project is for
Learning how a GPU actually draws a 3D object: from a text file of vertices to
shaded, textured pixels on screen, using an explicit modern API. The second goal
is to leave behind an `engine` crate (Vulkan setup, maths, mesh and image loading)
solid enough to be reused in ft_newton, ft_vox and ft_minecraft.

## What this project is NOT for
- **Maths in depth.** Vectors, matrices and projection are taught to "explain the
  logic and use it with ease" (T1). Quaternions, numerical stability and the
  derivations are for ft_newton.
- **Lighting and materials.** Not required by the subject. Basic lighting could be
  a bonus extra; physically based rendering belongs to a later project.
- **GPU memory allocators, multithreaded command recording, performance tuning.**
  scop draws one object; nothing here would show whether an optimisation mattered.
  ft_vox (huge numbers of chunks) is the right vehicle for this.
- **Large-scale engine architecture** (ECS, scene graphs, asset pipelines). The engine
  stays small and honest; it grows when a later project needs it.

## Constraints (subject v5.0)

**Mandatory**
- Parse a `.obj` file yourself and render it in a window **in perspective**.
- Rotate and translate the object around its three main axes, **origin at the
  object's centre** (the 42 logo must spin around its central axis, not a corner).
- Faces visually distinguishable through colour (per `.obj` face, or per triangle);
  shades of grey for the 42 logo.
- A key toggles a texture on/off with a **smooth transition** (no abrupt cut);
  the texture should be something cheerful (ponies, kittens, unicorns).
- Other `.obj` files will be tested at the defence.

**Technical constraints**
- Language: free (C, C++ or Rust preferred). API: OpenGL, Vulkan or Metal.
- One-line command to check for and install all dependencies (e.g. a Makefile).
- External libraries **only** for window and event management.
- **No external libraries** for `.obj` loading, matrices, or shader loading.
  (Image loading is not listed explicitly, but writing the texture loader yourself
  is the expected interpretation — treat it as forbidden.)

**README** — first line in italics: *This project has been created as part of the 42
curriculum by luferna3.* Plus Description, Instructions, Resources (with AI usage).

**Bonus** (only graded if the mandatory part is perfect)
- [10pt] Complex `.obj` files: concave and non-coplanar faces; render the original
  teapot without edge artefacts.
- [5pt] Texture without visible stretching on some faces.
- [10pt] Your own extra features — **to be decided later** (§ 19).

## Depth allocation

Scores 1–3 on centrality, transferability, cost of learning later. Goal is "build a
foundation", so concepts with transferability 3 get +1. Lucas's overrides applied:
maths capped at T1; pointers and windowing already known from C/C++, raylib and mlx.

| Concept | Cent. | Trans. | Cost later | Total | Tier |
|---|---|---|---|---|---|
| Rust ownership, borrowing, `Result`, `Drop` | 2 | 3 | 3 | 9 | T2 |
| `unsafe`, raw pointers, `ash` | 3 | 2 | 2 | 7 | T1 (pointers known) |
| How a GPU draws (rendering pipeline) | 3 | 3 | 3 | 10 | T2 |
| Vulkan instance, device, queues | 3 | 3 | 2 | 9 | T2 |
| Surface + swapchain | 3 | 3 | 2 | 9 | T2 |
| Command buffers + synchronisation | 3 | 3 | 3 | 10 | T2 |
| Shaders, GLSL → SPIR-V, graphics pipeline | 3 | 3 | 3 | 10 | T2 |
| Buffers + GPU memory | 3 | 3 | 3 | 10 | T2 |
| Engine/app architecture (reusable crate) | 2 | 3 | 3 | 9 | T2 (stated goal) |
| Vectors, matrices, perspective (own library) | 3 | 3 | 2 | 9 | **T1** (override) |
| Uniforms, push constants, descriptors | 3 | 3 | 2 | 9 | T2 |
| Depth buffer | 3 | 3 | 2 | 9 | T2 |
| `.obj` parser | 3 | 2 | 1 | 6 | T1 |
| Window + input (`winit`) | 3 | 2 | 1 | 6 | T1 (known from mlx/raylib) |
| Per-face colours, controls, centring | 3 | 1 | 1 | 5 | T1 |
| Own image loader (TGA) | 2 | 2 | 1 | 5 | T1 |
| Textures, samplers, UVs | 3 | 3 | 2 | 9 | T2 |
| Smooth colour↔texture transition | 3 | 1 | 1 | 5 | T1 |
| Bonus: concave/non-coplanar faces | 2 | 2 | 2 | 6 | T1 |
| Bonus: texture mapping without stretching | 2 | 2 | 2 | 6 | T1 |

**Vehicle note:** scop is an excellent vehicle for **the Vulkan object model and the
rendering pipeline** — one object, no distractions. It is a poor vehicle for
**performance** (one mesh), **maths depth** (only one camera and one model matrix)
and **lighting**. Those are deliberately left to ft_newton and ft_vox.

## Sections

| # | File | Title | Tier | Status | Checkpoint |
|---|---|---|---|---|---|
| 1 | `01-rust-for-c-programmers.md` | Rust for a C programmer | T2 | not started | Workspace (`engine` + `scop`), file reading with `Result` |
| 2 | `02-how-a-gpu-draws.md` | How a GPU draws a triangle | T2 | not started | `winit` window, event loop, Esc to quit |
| 3 | `03-vulkan-instance-and-device.md` | Talking to the GPU | T2 | not started | Instance + validation layers + logical device |
| 4 | `04-surface-and-swapchain.md` | Getting images to the screen | T2 | not started | Swapchain created, images listed |
| 5 | `05-commands-and-sync.md` | Giving the GPU work, and waiting | T2 | not started | Window cleared to a colour every frame |
| 6 | `06-shaders-and-pipeline.md` | Programs that run on the GPU | T2 | not started | Hard-coded triangle on screen |
| 7 | `07-buffers-and-gpu-memory.md` | Sending data to the GPU | T2 | not started | Triangle/quad from vertex + index buffers |
| 8 | `08-engine-architecture.md` | Splitting engine from app | T2 | not started | Refactor: scop uses only the engine's public API |
| 9 | `09-maths-for-3d.md` | Just enough 3D maths | T1 | not started | Own `Vec3`/`Mat4` + tests |
| 10 | `10-uniforms-and-descriptors.md` | Passing per-frame data to shaders | T2 | not started | Spinning cube in perspective |
| 11 | `11-depth-buffer.md` | What's in front of what | T2 | not started | Cube with correct face occlusion |
| 12 | `12-obj-parser.md` | Reading `.obj` files | T1 | not started | 42 logo and teapot load and render |
| 13 | `13-faces-and-controls.md` | Grey faces and moving the model | T1 | not started | Per-face greys, rotate/translate around centre |
| 14 | `14-textures.md` | Images on surfaces | T2 | not started | Own TGA loader, textured model |
| 15 | `15-smooth-transition.md` | Fading colour into texture | T1 | not started | Key toggles with smooth blend |
| 16 | `16-mandatory-finish.md` | Making the mandatory part perfect | T1 | not started | Resize handling, `make` one-liner, README, self-evaluation |
| 17 | `17-bonus-complex-obj.md` | Concave and non-flat faces | T1 | not started | Original teapot renders cleanly |
| 18 | `18-bonus-texture-mapping.md` | Textures without stretching | T1 | not started | No stretched faces |
| 19 | `19-bonus-extras.md` | Your own extras | TBD | not started | Decided later |

Sections 3–19 are written when you reach them, so the plan adapts to what turns
out to be hard.

## Defence prep
<!-- Populated as sections complete. Source: DECISIONS.md + section traps. -->

## Deferred / v2
- Lighting (candidate bonus extra, decided in § 19).
- Swapchain present-mode tuning, multiple frames in flight beyond 2.
