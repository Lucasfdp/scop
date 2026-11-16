# Decisions

One entry per real decision. These double as defence prep: "why did you choose this
over the obvious alternative?" is the question evaluators actually ask.

## 2026-11-02 — Rust + Vulkan over OpenGL
**Chose:** Rust with Vulkan (via MoltenVK on macOS).
**Rejected:** OpenGL 4.1 (much faster to first pixels, but a global-state model that
modern APIs have abandoned and that is deprecated on macOS); Metal (macOS-only, so
nothing transfers to Linux campus machines or future projects).
**Why:** more 3D/game projects are planned (ft_newton, ft_vox, ft_minecraft). Vulkan's
explicit model maps onto Metal, DX12 and WebGPU, and a reusable renderer is a goal.
**Cost accepted:** roughly 2–3× longer before the first frame; three new things at
once (Rust, Vulkan, 3D maths).
**Revisit if:** Vulkan cannot be installed on the evaluation machine.

## 2026-11-02 — `ash` for Vulkan bindings (not `vulkano`, not `wgpu`)
**Chose:** `ash` — thin, 1:1 bindings to the Vulkan C API.
**Rejected:** `vulkano` (a safe high-level wrapper that does real work for you —
risky under "external libraries only for window and event management");
`wgpu` (not Vulkan at all — it is its own API on top of several backends).
**Why:** `ash` adds no abstraction: every call is a Vulkan function. It is the
Rust equivalent of including `vulkan.h`.
**Cost accepted:** lots of `unsafe`; manual lifetime management of every Vulkan object.
**Revisit if:** an evaluator argues bindings count as an external library — be ready
to show that `ash` contains no rendering, loading or maths logic.

## 2026-11-02 — `winit` for window and events
**Chose:** `winit` (+ `raw-window-handle` to hand the window to Vulkan).
**Rejected:** GLFW via bindings (C dependency, extra install step); SDL (same).
**Why:** pure Rust, cross-platform, and exactly the category the subject allows.
**Cost accepted:** its event-loop API changes between versions — pin the version.

## 2026-11-02 — Cargo workspace: `engine` library + `scop` binary
**Chose:** split from day one into a reusable `engine` crate and a thin `scop` app.
**Rejected:** a single crate (simpler now, painful to extract later).
**Why:** stated goal — reuse the renderer, maths, `.obj` loader and image loader in
ft_newton, ft_vox and ft_minecraft.
**Cost accepted:** need to think about the public API boundary earlier than strictly
necessary; slightly more boilerplate.
**Revisit if:** the boundary keeps moving — wait until § 8 before freezing it.

## 2026-11-03 — Claude writes checkpoint code after the gate (method change)
**Chose:** owner reads each section and passes its gate; Claude then writes the
checkpoint code and explains it part by part; owner must explain it back.
**Rejected:** owner writes every line by hand (original plan) — owner found it too slow
and doesn't expect to hand-write code in practice.
**Why:** faster progress; understanding is still checked through gates and explain-back.
**Cost accepted:** weaker debugging muscle (Vulkan bugs are found by reading validation
errors and your own code); defence risk if any line can't be explained; README must
declare which parts were AI-generated (subject chapter II).
**Revisit if:** explain-back questions start getting missed, or debugging stalls.

## 2026-11-04 — GitHub issues as a view of the learning plan
**Chose:** one issue per section (workflow checklist in the body), labels for
status/tier/area/ES, one milestone per phase — generated and re-synced by
`scripts/learn_issues.py` from `00-PLAN.md`, `PROGRESS.md` and `learn/issues.json`.
**Rejected:** issues as the source of truth (state would split between GitHub and
`PROGRESS.md`); a GitHub Projects board (needs GraphQL, more setup for one person).
**Why:** a checkable, at-a-glance view without a second place to keep state in sync by hand.
**Cost accepted:** must run `sync` after updating `PROGRESS.md`; body checklists are
ticked by hand and are never rewritten by the script.
**Revisit if:** the checklists drift from reality, or a teammate joins.

## 2026-11-06 — `ash-window` for surface creation (not hand-written)
**Chose:** `ash-window` (pinned `=0.13.0`, with `raw-window-handle` `=0.6.2`): it lists
the window-system instance extensions and creates the `VkSurfaceKHR` from `winit`'s
window handles.
**Rejected:** writing surface creation by hand — on macOS that means attaching a
`CAMetalLayer` to the window through Objective-C calls, plus separate X11/Wayland code
for the campus machines. All platform glue, no rendering knowledge.
**Why:** it is window management — the one category the subject allows libraries for.
It contains no loading, maths, shader or rendering logic (two functions, ~200 lines,
maintained by the `ash` authors).
**Defence argument:** "A surface is the link between Vulkan and the OS window; this
crate only translates winit's window handle into that link. Everything Vulkan draws
is my code."
**Cost accepted:** one more dependency an evaluator might question.
**Revisit if:** an evaluator rejects it, or out of interest later — write the macOS
path by hand.

## 2026-11-07 — Dynamic rendering (not render passes)
**Chose:** `VK_KHR_dynamic_rendering` (an extension on MoltenVK's Vulkan 1.2; core from
1.3): drawing starts with `cmd_begin_rendering` naming the image view directly.
**Rejected:** `VkRenderPass` + one `VkFramebuffer` per swapchain image (Vulkan 1.0 style).
**Why:** fewer objects; nothing extra to rebuild on resize (§ 16); pipelines only need the
attachment *formats*; continues § 5's hand-written layout barriers; the direction Vulkan
and modern engines have taken — better for ft_vox / ft_minecraft (shadow maps,
post-processing = more passes).
**Cost accepted:** needs driver support (checked at startup, clear error if missing);
gives up subpasses (a tile-GPU optimisation that matters on mobile, not here).
**Confirmed:** `vkcube` runs on the 42 Madrid campus machines (2026-11-07).
**Revisit if:** a campus driver lacks the extension.

## 2026-11-07 — Shaders compiled by `build.rs` with `glslc`, embedded in the binary
**Chose:** a Cargo build script runs `glslc` (GLSL → SPIR-V) on every shader in
`scop/shaders/`; the bytes are embedded with `include_bytes!` and turned into SPIR-V
words by our own loader (no `ash::util::read_spv`).
**Rejected:** a Makefile-only compile step (`cargo run` alone would use stale shaders);
committing pre-built `.spv` files (easy to forget to rebuild).
**Why:** `cargo build` can't forget; no runtime file paths to break at the defence.
**Defence argument:** `glslc` is a compiler run at build time, like `rustc` — not a
library linked into scop. Shader *loading* (bytes → validated SPIR-V words → shader
module) is our own code, as the subject requires.
**Cost accepted:** `glslc` must be installed (the Makefile installs it, § 16).

## 2026-11-08 — Mesh uploads go through a staging buffer into device-local memory
**Chose:** write vertices/indices into a `HOST_VISIBLE | HOST_COHERENT` staging buffer,
copy with `cmd_copy_buffer` into a `DEVICE_LOCAL` buffer, wait, destroy the staging buffer.
**Rejected:** drawing straight from host-visible memory.
**Why:** fastest to read on every GPU (campus graphics cards keep host-visible memory on
the CPU side); § 14 textures need the same staging path anyway. On the M2 (unified
memory) the only cost is one extra copy at load time.
**Cost accepted:** more code (one-off command buffer, fence wait) for each upload.
**Revisit if:** per-frame data — that goes in host-visible memory (§ 10's uniforms).

## 2026-11-08 — Apps plug into the engine through an `App` trait
**Chose:** `engine::run(config, app)` with `trait App { setup, update, draw }`; the engine
owns the loop and calls the app (inversion of control). GPU objects stay in the engine,
apps hold `MeshId` / `PipelineId` handles; input reaches apps as the engine's own `Key`.
**Rejected:** a struct of callbacks (state awkward to share; every new callback breaks
every app); the app owning the loop (fights `winit` 0.30's callback model).
**Why:** app state lives naturally in `self`; new engine features arrive as default
methods without breaking existing apps; apps never depend on `ash` or `winit`.
**Cost accepted:** a fixed call order (setup → update → draw) that apps must fit into.
**Revisit if:** a project needs several windows or its own loop timing (ft_vox?).

## 2026-11-11 — Camera in a per-slot uniform buffer, model matrix as a push constant
**Chose:** `projection × view` in one persistently mapped `HOST_VISIBLE | HOST_COHERENT`
uniform buffer **per frame slot**, reached through a descriptor set (set 0, binding 0);
each draw's model matrix as a 64-byte push constant. One pipeline layout shared by all
engine pipelines.
**Rejected:** both matrices as push constants (fits in 128 bytes, but fills the guaranteed
space and postpones descriptors to § 14's harder case); both in uniform buffers
(per-object regions or dynamic offsets — bookkeeping for nothing).
**Why:** each value sits where its update rate says it belongs (per-frame vs per-draw);
descriptors are learned on the simplest resource first; the pattern carries to ft_vox.
**Cost accepted:** more Vulkan objects (set layout, pool, sets, two buffers) to create and destroy.
**Revisit if:** per-draw data grows past ~64 bytes (move it to a buffer with dynamic offsets).

## 2026-11-14 — One depth image with a per-frame barrier; back-face culling off
**Chose:** a single swapchain-sized `D32_SFLOAT` depth image (first supported of
`D32_SFLOAT`, `D32_SFLOAT_S8_UINT`, `D24_UNORM_S8_UINT`), moved to
`DEPTH_STENCIL_ATTACHMENT_OPTIMAL` by a barrier each frame that waits for the previous
frame's depth writes; cleared to 1.0, store `DONT_CARE`, compare `LESS`.
Back-face culling stays off.
**Rejected:** one depth image per frame slot (no inter-frame barrier, but twice the
memory and more to rebuild on resize); culling on (faster, but `.obj` files don't
guarantee consistent winding — a mixed-winding model would lose faces).
**Why:** the standard setup; correct for any model the evaluator loads.
**Cost accepted:** frames in flight serialise on the depth image (negligible on one
queue); hidden back faces are still shaded before the depth test can drop them where
early tests don't apply.
**Revisit if:** ft_vox (own meshes, consistent winding → enable culling).

## 2026-11-14 — `.obj` parser in the engine, `v` + `f` only, fan triangulation
**Chose:** `engine::parse_obj(text) -> Result<ObjModel, ObjError>` reading `v` and `f`
(any corner count, `a/b/c` tokens, negative indices), fan-triangulating faces, and
computing the bounding box; every other line skipped. Errors carry the line number.
Colours are chosen by scop, not the parser.
**Rejected:** reading `.mtl` materials (the subject asks for greys); a parser in scop
(ft_newton / ft_vox will load `.obj` too); ear-clipping now (needed only for concave
faces — the § 17 bonus).
**Why:** smallest parser that loads every mandatory file and fails cleanly on bad ones.
**Cost accepted:** concave faces may draw stray triangles until § 17; `vt`/`vn` ignored
until needed.
**Revisit if:** § 17 (concave faces), § 18 (file texture coordinates).

## 2026-11-14 — One grey per `.obj` face via duplicated corners; keyboard controls
**Chose:** the parser records each triangle's source face (`ObjModel::triangle_faces`);
scop gives every triangle its own three corners, coloured `0.25 + 0.5 × fract(face ×
0.618…)`. Controls: arrows / W S move (screen axes, × radius × dt), I K / J L / U O turn
around x / y / z, Space toggles the automatic spin, R resets. Model matrix:
`translation(position) × rotation_y × rotation_x × rotation_z × translation(−centre)`.
**Rejected:** grey per triangle (quads show a diagonal seam); `gl_PrimitiveID` in the
fragment shader (needs the geometry-shader feature, unsupported on MoltenVK); quaternions
(more than a viewer needs).
**Why:** flat, distinct, subtle faces as the subject asks; every control maps to one
card of the § 9 recipe.
**Cost accepted:** 3× the vertices (18,960 for the teapot — negligible); gimbal lock at
some Euler angle combinations.
**Revisit if:** lighting arrives (normals also need duplicated corners — same layout).

## 2026-11-15 — Textures: own TGA loader, set 1 per texture, planar UVs, embedded default
**Chose:** `engine::load_tga` (types 2 and 10, 24/32-bit, both origins) → RGBA;
`R8G8B8A8_SRGB` `DEVICE_LOCAL` image via staging with two layout transitions; one shared
`LINEAR`/`REPEAT` sampler; each texture its own descriptor set (set 1, combined image
sampler, fragment stage); a 1×1 white default texture; UVs made by scop by planar
projection along the bounding box's thinnest axis; the texture embedded with
`include_bytes!`.
**Rejected:** PNG (needs a hand-written decompressor), BMP (padding + header versions),
PPM (few tools export it); texture in set 0 (different update rate from the camera);
reading `vt` (the subject's files have none); a run-time path (breaks when run from
another directory).
**Why:** smallest loader real tools feed; descriptors split by update rate (§ 10's rule).
**Cost accepted:** stretched streaks on faces parallel to the projection (the § 18 bonus);
no mipmaps (shimmer when far away).
**Revisit if:** § 18 (stretch-free mapping); ft_vox (texture atlases, mipmaps).

## 2026-11-15 — Smooth texture toggle: target + dt stepping, smoothstep, blend in the push constant
**Chose:** T flips a target (0 grey / 1 picture); every frame the raw progress moves
towards it by `dt / 0.6 s`, clamped; scop eases it with smoothstep and sends it as a
`float` after the model matrix in the push constant (one range, `VERTEX | FRAGMENT`,
68 bytes); the fragment shader does `mix(grey, picture, blend)`.
**Rejected:** a timer restarted on each press (jumps when pressed mid-fade); the blend in
the camera uniform buffer (it's per draw, not per frame); a linear fade (abrupt start/stop).
**Why:** reversible at any moment, frame-rate independent, no abrupt cuts.
**Cost accepted:** the push block grows to 68 of the guaranteed 128 bytes.
**Revisit if:** more per-draw data appears (move it to a buffer with dynamic offsets).

## 2026-11-16 — Resize by full rebuild after `device_wait_idle`; `make` + `scripts/deps.sh`
**Chose:** on `Resized` (non-zero), out-of-date or suboptimal: `device_wait_idle`, destroy
and recreate the swapchain and depth image at the window's current size. `make` runs a
dependency script (Rust via rustup in the home folder; `glslc` / Vulkan via Homebrew on
macOS, reported with the install command on Linux) and a release build.
**Rejected:** recreating with `old_swapchain` while frames are still in flight (smoother
during a drag, but more sync to reason about for a rare event); a script that tries `sudo`
(fails or prompts on campus machines).
**Why:** simplest correct resize; a setup command that never pretends to have installed
something it couldn't.
**Cost accepted:** a brief GPU pause on each resize; on campus Linux, missing system
packages must be installed by hand.
**Revisit if:** live-resize smoothness matters (ft_vox).
**Update (same day, owner's choice — option C):** campus machines allow `curl`/`pip` but not
admin installs. So: compiled `.spv` files are committed as a fallback and `build.rs` uses
`glslc` when present, else the committed ones (with a warning; a test checks they're not
stale); and `deps.sh` offers to `curl` the LunarG Vulkan SDK into the home folder when
`glslc` is missing. Rejected: A alone (a 250 MB download blocks the build), B alone (no
real compiler on campus). Cost: `.spv` must be refreshed when shaders change (the test
catches it); this reverses the 2026-11-07 "don't commit `.spv`" choice for the fallback copy.
