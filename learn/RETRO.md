# scop — Retrospective

## What I can do now that I couldn't before

- **Stand up Vulkan from nothing**: instance, validation layers, device, surface, swapchain,
  command buffers, a frame loop with two frames in flight — on macOS through MoltenVK, and on the
  campus Linux machines.
- **Explain GPU synchronisation by who waits for whom**: fences (the CPU waits), semaphores (the GPU
  waits), barriers (later commands wait) — and spot the silent races (a shared camera buffer, a
  missing depth barrier) that validation never reports.
- **Get data onto the GPU the right way**: staging into device-local memory for things written
  once, mapped host-visible memory for things written every frame; descriptors as pointers, push
  constants as sticky notes.
- **Write and wire shaders**: GLSL → SPIR-V at build time, my own SPIR-V loader, pipelines with
  dynamic state, hot reload while running.
- **Do the 3D maths I need, without deriving it**: my own `Vec3`/`Mat4` with tests, cards chained
  right to left, perspective through `w`, look-at, orbit camera with a pitch clamp.
- **Read files by hand**: an `.obj` parser (quads, n-gons, negative indices, `1/2/3` tokens, ear
  clipping for concave faces) and a TGA loader (24/32-bit, RLE, both origins) — no libraries.
- **Render beyond one model**: depth testing, textures with correct sRGB, flat lighting, instancing
  (512 copies, one draw), a cube-map sky, chrome, live reflections rendered to a texture, text on
  screen.
- **Write Rust comfortably as a C programmer**: ownership instead of `free`, `Drop` for destroy
  order, `Result` + `?`, traits, closures as callbacks, workspaces.
- **Split engine from app**: a reusable `engine` crate scop uses only through its public API —
  and the two reuse questions for deciding what goes where.

## What stayed shallow on purpose

- **The maths** — capped at T1 from the start (use it, test it, don't derive it); §§ 20–25 had no
  maths at all. Quaternions, the inverse-transpose for stretched models, and why the perspective
  matrix has its exact entries are all known names, not understood formulas.
- **OS / driver detail** — e.g. Q-3.2 kept to one sentence (why MoltenVK needs the portability
  opt-in), by choice.
- **Performance** — one probe, fixed-size buffers, `device_wait_idle` wherever things are destroyed,
  vsync always on. Fine for scop; I never measured real GPU cost.
- **Lighting** — one directional light, flat shading, no shadows, no specular.
- **Allocation** — one memory allocation per buffer; no allocator.

## What was harder than the plan assumed

- **Synchronisation (§ 5)** — frames vs frame slots vs swapchain images needed an extra explainer;
  "who waits for what" came back as a miss in §§ 7, 10 and 16 before it stuck (clean by § 19).
- **sRGB (§ 14)** — I predicted "too dark" for a `UNORM` texture; it's too pale (double-encoded).
  Decode-in / encode-out took fresh variants to settle.
- **Ear clipping (§ 17)** — I first thought the ear was the piece thrown away; it took working the
  L-shape by hand. And the section file itself was lost in a commit and had to be rewritten.
- **Build time vs run time (§ 6)** — I swapped them twice ("debug build" ≠ "build time").
- **Matrix memory layout (§ 9)** — row/column on paper vs in memory, and what a transposed matrix
  actually does (feeds `w`, warps the model).
- **Inside-out vs upside-down (§ 19)** and **cube maps as pictures, not directions (§ 23)** — both
  needed re-explaining with a picture.
- **The Spanish keyboard** — `+`/`−` didn't register, so zoom/dolly moved to W/S.
- **Campus portability (§ 16)** — no `glslc` and no admin rights: led to the committed `.spv`
  fallback and the LunarG SDK download in `deps.sh`.

## What the next project should pick up

From the "Revisit if" lines in `DECISIONS.md` and the deferred list in `00-PLAN.md`:

- **ft_vox**: a per-frame-slot **deletion queue** (meshes swapped every frame); a real memory
  **allocator** and skipping staging on unified memory / Resizable BAR; **back-face culling** once
  meshes have consistent winding; **texture atlases and mipmaps**; growing instance buffers;
  smoother live resize; **shadows**.
- **ft_newton**: **quaternions** for free rotation; watertight (welded) meshes for physics; the
  inverse-transpose for normals on stretched models.
- **Rendering upgrades whenever wanted**: smooth (averaged) normals → smooth shading and smooth
  chrome; triplanar mapping for organic models; several reflection probes; HDR skies; a bigger font
  atlas for accented text.
- **The engine itself** — camera, transforms, tweens, mesh builder, hot reload, sky, default
  shaders, reflection probe, text overlay — is ready to be the base of the Rust engine.
- **The review queue** still has ~30 items with 0 clean recalls: worth a pass before the defence.

## Notes on the plan itself

- **Changing the method on 2026-11-03 was the best call**: answering the gate first, then Claude
  writing the checkpoint and me explaining it back, kept the understanding without the hand-typing.
  The explain-backs caught real gaps (bind vs write, `+= gap` vs `+= max_step`, matrix order in
  instancing).
- **Fresh variants after a miss** worked better than re-reading the answer — most misses passed
  on the second, differently worded question.
- **Lowering §§ 20 and 22 to T1 / no maths** fit what I already knew from rubik and cubed; the gate
  questions about behaviour and setup were enough.
- **Bonus extras grew to six sections (§§ 19–25)** — far more than the 10-point bonus needs, on
  purpose, as engine groundwork. § 24 (moving it all into the engine) was the right way to close.
- **Pausing the Spanish mirror** saved time mid-project; catching it up in batches worked fine.
- **Lesson for next time**: commit the section file *before* marking a section done — three files
  were emptied by "completed" commits, and § 17's text had to be rebuilt from the code.
- **The review queue was filled but rarely revisited** — next plan, schedule review sessions
  instead of leaving them to "next session".
