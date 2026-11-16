# Progress

**Current section:** 16
**Last session:** 2026-11-11

## Sections

| # | Status | Gate | Date | ES synced | Notes |
|---|---|---|---|---|---|
| 1 | done | passed after misses (dangling raw pointer, local drop order, stdout/stderr redirect) — checkpoint code by Claude, explained back | 2026-11-04 | yes | Owner self-checked vs ANSWERS first; gate re-run with fresh questions |
| 2 | done | passed; explain-back corrected: trait signatures are fixed, `From` impl (not App) does error conversion | 2026-11-04 | yes | Checkpoint code by Claude (winit 0.30.13); verified on Mac (Esc, keys, resize) |
| 3 | done | passed after misses (destroy order; portability opt-in; Clone = copied handle → double destroy) — checkpoint code by Claude, verified on Mac (GPU line, validation leak test); explain-back passed with misses (only `?` calls `From`; get/enumerate vs create; push_next timing; queue-priorities list) | 2026-11-06 | paused | Q-3.2 narrowed to one sentence by owner's choice (no OS-level detail) |
| 4 | done | passed after misses (why surfaces are an extension; request min+1 not 0; physical vs logical extent; 0×0 extent on minimise; instance needs window-system extensions) — checkpoint code by Claude, verified on Mac (B8G8R8A8_SRGB, FIFO, 2560x1440, 3 images) ; explain-back passed with misses (present check → swapchain creation; exists vs enabled; image-count clamp) | 2026-11-06 | paused | |
| 5 | done | passed after misses (semaphores: CPU never waits; why unsignalled fence hangs; reset_fences between acquire and submit; final barrier = layout change) — checkpoint code by Claude, verified on Mac (pulsing blue, clean exit, unsignalled-fence hang test) ; explain-back passed with misses (access mask = what the next/previous work does to memory; render_done per image because the screen holds it; device_wait_idle = 'in use', not order) | 2026-11-07 | paused | Added 'frames, frame slots and images' explainer after confusion |
| 6 | done | passed after misses (rasteriser interpolates, not the fragment shader; size must match = viewport/scissor ← extent; build time vs run time: glslc errors stop the build) — checkpoint code by Claude verified on Mac (RGB triangle, write-mask test) ; explain-back passed with misses (include_bytes! copies bytes, not the path; length check guards words[0]; pipeline compiles its own copy of the shaders; wait at the first stage that touches the image) | 2026-11-07 | paused | Decisions: dynamic rendering, build.rs + glslc |
| 7 | done | passed after misses (usage = what the GPU may do; 6 indices, corners stored once; submit returns before the copy runs; memory vs usage flags vs index type) — checkpoint code by Claude verified on Mac (4-colour quad, clean exit, index test) ; explain-back passed with misses (memory_type_bits = allowed types; upload fence/barrier vs frame loop; struct literals need every field) | 2026-11-08 | paused | Decision: staging → device-local |
| 8 | done | passed after misses (handles stop GPU use-after-free, not just lost references; pub = scop writes the name, pub(crate) = other engine modules, private = this file) — checkpoint code by Claude ; explain-back passed (dt explained on request; validate ids before acquire/reset_fences — answer echoed the walkthrough, check again) | 2026-11-08 | paused | Decision: App trait |
| 9 | done | passed after misses (why 4×4: 3×3 has no slot for a move; cards chain with ×, swap whole cards not their arguments; flipped matrix: move feeds w per corner → warped, not just smaller) — checkpoint code by Claude (maths.rs, 18 tests, verified on Mac) ; explain-back passed with misses (private = this file, not the engine; paper [row][col] vs memory [col][row]; `+1` in row 3 gives w = −4, not −1; y-flip test divides by w, not y) | 2026-11-11 | paused | Walked through in 5 stops (w, cards as code lines, order, perspective, memory layout) |
| 10 | done | passed after misses (Q-10.1 needed the 'why': update rate / who shares it; Q-10.2 thought the shared buffer gets destroyed + validation catches it — it's overwritten mid-read, a silent race; Q-10.3 took `1.0` for the distance — it's w = point going in; Q-10.4 descriptor = pointer, write only for a different resource; Q-10.5 unsure stretch vs squash — stretched sideways) — checkpoint code by Claude (shader_inputs.rs, push constants, spinning cube) verified on Mac (clean run, cube spins) ; explain-back passed with misses (bind ≠ write: `update_descriptor_sets` once at startup, `cmd_bind_descriptor_sets` per frame — `cmd_` = recorded each frame; staging is chosen by how often the CPU writes, not error-checking — camera written every frame, mesh once; `ShaderInputs` owns the pipeline layout, not shader bytes → pipelines destroyed first) | 2026-11-13 | paused | Section written 2026-11-11 |
| 11 | done | passed with small misses (Q-11.1: first comparison is against the cleared 1.0, not 0; Q-11.3: near = 0.1 is a distance, after projection depths are 0..1 with near → 0; Q-11.2 added a wrong side claim that depth-write-off causes validation errors) — checkpoint code by Claude (depth.rs, depth test LESS, per-frame barrier + cleared attachment) verified on Mac (no back faces, GREATER → background only) ; explain-back passed with misses (barrier missing → frame N+1's clear races frame N's tests: one-frame flicker, silent; depth image must cover every pixel of the render area — smaller = validation error / pixels with no depth slot) | 2026-11-14 | paused | Section written 2026-11-14 |
| 12 | done | passed after misses (Q-12.2: fan starts from the first corner on the line, not the one nearest the origin — fresh variant passed; Q-12.5 asked for the 2-card at-origin version; Q-12.1 forgot the skipped lines) — checkpoint code by Claude (engine obj.rs: parse_obj, ObjModel, ObjError; scop loads/centres/frames the model) verified on Mac (42.obj, teapot, teapot2 render upright, clean) ; explain-back passed (added: positions.len() also resolves negative indices; line-free helpers are simpler/testable) | 2026-11-14 | paused | Section written 2026-11-14 |
| 13 | done | passed after misses (Q-13.1: shared corners don't distinguish faces at all — greys bleed from each corner; Q-13.3: held on a toggle flips every frame, not 'spin while held' — both fresh variants passed; Q-13.4: consequence filled in (position gets turned → moves away / orbits); Q-13.5: radius scales *movement*, 0.2 = fraction of model size per second) — checkpoint code by Claude (ObjModel::triangle_faces; grey_faces duplicated corners + face_grey; Placement + 5-card matrix; key controls) verified on Mac (greys, all keys; +/− didn't register on Spanish layout → switched to W/S) ; explain-back passed with misses (old .obj indices on duplicated vertices = in range but wrong corners, silent — not 'running out'; `*p =` writes through the reference; `&mut` = arrow to the original vs no `&` = copy; spin + angles.y: Space stops `spin` growing, nothing resets) | 2026-11-14 | paused | Section written 2026-11-14 |
| 14 | done | passed after misses (Q-14.4: staging is worth it *because* the CPU writes once; Q-14.5: thought UNORM texture → too dark — it's too pale (no decode in, still encode out = double-encoded); fresh variants passed: SRGB tex + UNORM swapchain = too dark, both UNORM = looks right) — checkpoint code by Claude (tga.rs loader; texture.rs upload + 2 transitions + sampler + set 1 + white texture 0; run_once helper; planar UVs; embedded texture) verified on Mac (owner's reaper-turtle picture upright, correct colours, streaks on thin sides) ; explain-back passed (first_set must match `set = N`; white = 1.0 per channel, neutral when multiplying — not 'the base colour') | 2026-11-15 | paused | Section written 2026-11-15 |
| 15 | done | passed after misses (Q-15.2: swapped roles — key flips the target, blend steps towards it; Q-15.4: error at pipeline creation, not cmd_push_constants — both fresh variants passed; Q-15.3/15.5 precisions: reverses from where it is; smoothstep changes the pace) — checkpoint code by Claude (push block 68 bytes VERTEX|FRAGMENT, DrawCall, mix in frag, Fade target/progress + clamp + smoothstep, T key) verified on Mac (fades both ways, reverses mid-fade) ; explain-back passed with misses (`+= gap` snaps in one frame — it doesn't overshoot; `+= max_step` is the one that overshoots; easing is applied once on output to a linearly stepped progress) | 2026-11-15 | paused | Section written 2026-11-15 |
| 16 | gate passed | passed after misses (Q-16.1 needed 'why no pipeline rebuild' — dynamic viewport/scissor; Q-16.2: device_wait_idle = CPU waits for the GPU's queue to drain, it doesn't tell the GPU to stop — fresh variant passed; Q-16.3: decision option C taken (committed .spv fallback + curl LunarG SDK)) — checkpoint code pending | 2026-11-16 | paused | Section written 2026-11-16 |
| 17–19 | not written | — | | — | Written on arrival |

## Prerequisite audit (2026-11-02)
- Rust: new. C/C++ and pointers: known.
- 3D rendering, GPU APIs, 3D maths: new (all 8 diagnostic questions "no idea").
- Windows/events: known via raylib and mlx.
- Maths depth: capped at T1 by owner's choice — explain the logic, use with ease.

## Overrides
- 2026-11-04: § 3 Q-3.2 kept at one-sentence depth (owner doesn't want OS/driver detail).
- 2026-11-03: method changed — Claude writes checkpoint code after each gate, owner explains it back (see DECISIONS.md).

## Review queue

| Question | Source | Clean recalls | Next due |
|---|---|---|---|
| device_wait_idle: who waits for what (CPU waits until the GPU's submitted work is done) — fresh variant | § 16 | 0 | after § 18 |
| Target vs value: the key flips the target, the value steps towards it each frame (reversible, no jump) — fresh variant | § 15 | 0 | after § 17 |
| sRGB: decode on texture read (darkens) + encode on swapchain write (brightens) must pair up — fresh variant | § 14 | 0 | after § 16 |
| Rust references: `let p = &mut x` is an arrow to x (changes reach x) vs `let p = x` is a copy; `*p = v` replaces the whole value — fresh variant | § 13 explain-back | 0 | after § 15 |
| held vs pressed: toggles need `pressed` (held flips every frame) — fresh variant | § 13 | 0 | after § 15 |
| Fan triangulation: first corner on the `f` line + each next edge, then −1 — fresh variant | § 12 | 0 | after § 14 |
| Why the depth image must match the swapchain size (one depth slot per pixel; render area ≤ attachment size) — fresh variant | § 11 explain-back | 0 | after § 13 |
| Depth: cleared to 1.0, LESS wins; distances (near = 0.1) vs depth values (0..1 after projection) — fresh variant | § 11 | 0 | after § 13 |
| `update_descriptor_sets` (aim the pointer, once) vs `cmd_bind_descriptor_sets` (pass it in, every frame — `cmd_` rule) — fresh variant | § 10 explain-back | 0 | after § 12 |
| Staging vs direct HOST_VISIBLE: decided by how often the CPU writes (once → staging, every frame → direct) — fresh variant | § 10 explain-back | 0 | after § 12 |
| Shared camera buffer across slots: overwritten while the GPU reads it → one-frame jumps; validation silent (race) — fresh variant | § 10 | 0 | after § 12 |
| Descriptor = pointer: new values → write the buffer; different resource (texture, new buffer) → descriptor write — fresh variant | § 10 | 0 | after § 12 |
| Perspective divide is by w (= distance), not by y; why row 3 has −1 (camera looks down −z → negative w would be culled) — fresh variant | § 9 explain-back | 0 | after § 11 |
| Why 4×4 and not 3×3: a 3×3 mix of x,y,z can never move (0,0,0); the 4th column holds the move, × w — fresh variant | § 9 | 0 | after § 11 |
| Chaining cards: × (never +), right to left, swap whole cards not their arguments — fresh variant | § 9 | 0 | after § 11 |
| Row- vs column-major mix-up: the move lands in the w row → w differs per corner → warped/garbage, model doesn't move — fresh variant | § 9 | 0 | after § 11 |
| Why the portability extension + flag exist (one sentence: loader hides MoltenVK unless you opt in) — fresh variant | § 3 | 0 | after § 5 |
| Why validate draw ids before acquire/reset_fences (half-done frame = fence off with no job → hang) — fresh variant | § 8 explain-back | 0 | after § 10 |
| pub vs pub(crate) vs private — who writes the name (fresh variant) | § 8 | 0 | after § 10 |
| Upload vs frame loop: the upload's own fence (CPU: copy done) and barrier (copy visible to VERTEX_INPUT reads) are separate from § 5's per-frame sync — fresh variant; MISSED 2026-11-12 (mixed fence with semaphore wait stage; thought frame fence guards the image), then fresh variant passed after re-explain | § 7 explain-back | 1 (2026-11-12) | after § 12 |
| Memory flags vs usage flags vs index type — which goes where (fresh variant) | § 7 | 1 (2026-11-12) | after § 12 |
| Upload is its own submit: staging stays in use until the copy's fence (fresh variant) | § 7 | 0 | after § 9 |
| `include_bytes!` embeds bytes vs reading a path at run time — fresh variant | § 6 explain-back | 0 | after § 8 |
| Why shader modules can be destroyed after pipeline creation (driver compiles its own copy) — fresh variant | § 6 explain-back | 0 | after § 8 |
| Semaphore wait stage = first stage that touches the image — fresh variant | § 6 explain-back | 0 | after § 8 |
| Who interpolates (rasteriser) vs who decides the final colour (fragment shader) — fresh variant | § 6 | 1 (2026-11-08) | after § 11 |
| Build time (glslc, rustc) vs run time (validation, hangs) — fresh variant; MISSED again 2026-11-08 (swapped them: confuses "debug build" with "build time") | § 6 | 1 (2026-11-12) | after § 12 |
| Access masks: barrier 1 dst `TRANSFER_WRITE` / barrier 2 src `TRANSFER_WRITE` — what each orders — fresh variant | § 5 explain-back | 0 | after § 7 |
| Why `device_wait_idle` before teardown (objects still *in use*, not wrong order) — fresh variant | § 5 explain-back | 0 | after § 7 |
| Where `reset_fences` goes and why (after acquire, before submit) — fresh variant | § 5 | 0 | after § 7 |
| Per slot vs per image: fence/command buffer/image_free vs render_done/view — fresh variant | § 5 | 0 | after § 7 |
| Layout transitions are recorded pipeline barriers (TRANSFER_DST_OPTIMAL → PRESENT_SRC_KHR) — fresh variant | § 5 | 0 | after § 7 |
| Draw-capable ≠ present-capable: which step fails if you skip the present check (swapchain creation) — fresh variant | § 4 explain-back | 0 | after § 6 |
| Device extension exists vs enabled: why `create_device` must list `VK_KHR_swapchain` — fresh variant, own words | § 4 explain-back | 0 | after § 6 |
| `choose_image_count` clamping: min+1, then `.min(max)` only if max > 0 — fresh variant | § 4 explain-back | 0 | after § 6 |
| Minimised window = 0×0 extent → skip render/rebuild (1×1 is valid) — fresh variant | § 4 | 0 | after § 6 |
| Two steps that need the window: instance extensions (display handle), present-capable queue family (surface) — fresh variant | § 4 | 0 | after § 6 |
| `queue_priorities` list: its length = how many queues; each value = one queue's importance (0.0–1.0); valid `get_device_queue` indices — fresh variant | § 3 explain-back | 0 | after § 4 |
| Which calls you must destroy (`create_*`/`allocate_*`) vs not (`get_*`/`enumerate_*`) — fresh variant | § 3 explain-back | 0 | after § 5 |
| Only `?` calls `From`; `return Err(x)` needs x already the right type — fresh variant | § 3 explain-back | 0 | after § 5 |
| Move vs clone: a move *transfers* ownership (old variable unusable), `Clone` *duplicates* it — fresh variant | § 3 | 0 | after § 5 |
| Q-1.5 (dangling CString pointer — ask a fresh variant) | § 1 | 1 (2026-11-08; said "end of scope" — it is end of statement) | after § 10 |
| Stdout vs stderr redirect (`>` vs `2>`) — fresh variant | § 1 | 1 (2026-11-06, § 5 walkthrough: println vs eprintln with `2>`) | after § 8 |
| Who converts errors for `?` (`From` impl) — fresh variant | § 2 | 0 | after § 4 |
| Q-1.3 (drop order fields vs locals — fresh variant) | § 1 | 1 (2026-11-08) | after § 10 |

## Open threads
- 2026-11-04: Spanish (learn_es/) paused from § 3 on, by owner's choice — catch up all sections at the end. `check_learn.py` parity errors for § 3+ are expected until then.
- § 1 weak spots: dangling raw pointer from a dropped local; drop order of locals vs struct fields.
- ✅ 2026-11-07: `vkcube` runs on the 42 Madrid campus machines.
- MoltenVK reports **Vulkan 1.2.357** on the M2. Decided 2026-11-07: dynamic rendering (via `VK_KHR_dynamic_rendering`) — see DECISIONS.md.
- Bonus extras (§ 19): deliberately undecided.
