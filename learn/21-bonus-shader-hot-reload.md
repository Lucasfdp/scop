---
section: 21
slug: bonus-shader-hot-reload
tier: T1
status: done
prereqs: [6, 16, 19]
terms: [hot reload, runtime shader compile, modification time, polling, pipeline swap, shader contract]
---

# Section 21 — Shaders that reload while running (bonus)

**Depth: T1** — almost all reuse: § 6 (shaders become pipelines), § 16 (`glslc`, the `.spv`
fallback), § 19 (new-before-old, waiting before destroying). Bonus extra (owner's pick,
2026-11-18): **edit a shader, see the change without restarting**.
**You'll be able to:** keep scop open, change `mesh.frag` in your editor, save, and see the new
colours a moment later — or, if the shader is broken, read the compiler's error while the old
version keeps running.

---

## The idea

Until now shaders are compiled once, **before** scop is built (§ 16's `build.rs`), and the
SPIR-V bytes are baked into the binary. Changing a colour means: edit, rebuild, restart, find
your way back to the same view. Hot reload removes the last three steps.

It's three small jobs:

1. **Notice the change.** Every quarter of a second, scop asks the file system when
   `mesh.vert` and `mesh.frag` were last changed (their **modification time**). A newer time
   than last check = the file was saved. Asking repeatedly like this is **polling**.
2. **Compile it.** Vulkan only accepts SPIR-V (§ 6), so scop runs the same `glslc` the build
   uses — now while scop is running — and takes the bytes it produces.
3. **Swap the pipeline.** A pipeline holds its own compiled copy of the shaders (§ 6), so new
   shaders mean new pipelines. Same order as § 19: build the **new** ones first; only if
   that worked, wait for the GPU and destroy the old ones. The new pipeline goes into the old
   one's slot, so scop's `PipelineId`s keep working.

If anything fails — `glslc` reports an error, or the pipeline can't be built — scop prints
the message and **keeps drawing with the old shaders**. A typo never closes the viewer.

**What it can't change: the contract.** A shader and the Rust side agree on what goes in and
out: the vertex layout (§ 7), the camera block (§ 10), the push block (§ 15), the texture
(§ 14). Hot reload can change anything *inside* the shader — colours, formulas, the debug
modes — but a new input the Rust side doesn't send still needs a Rust change and a restart
(Q-21.4).

**Where the analogy stops being true:** "save and see it" makes it feel like editing a web page.
But the reload is only as good as `glslc` being installed: on a machine without it (the campus
fallback, § 16), scop says hot reload is off and carries on with the built-in shaders.

---

## The names for it

**hot reload** — replacing part of a running program (here: its shaders) without restarting it.

**runtime shader compile** — running `glslc` from inside scop (`std::process::Command`), reading
its SPIR-V from the output. Same tool as the build, different moment.

**modification time** — when a file was last written, as the file system records it ("mtime").
Saving changes it; reading doesn't.

**polling** — checking repeatedly ("has it changed yet?") instead of being told. Simple, and
cheap at four checks a second.

**pipeline swap** — build the new pipeline, wait for the GPU, destroy the old one, put the new
one in its slot. The id stays the same: "scop's solid pipeline", whichever version.

**shader contract** — everything a shader expects the Rust side to provide (vertex inputs,
uniform and push blocks, textures). Hot reload works inside it, never across it.

---

## How it actually works

### 1. Engine: pipelines from bytes, replaced in place

```
Shaders<'a> { vertex: &'a [u8], fragment: &'a [u8] }   // was &'static: runtime bytes are owned
Gfx::replace_pipeline(id, shaders, style)
    new = Pipeline::new(…)          → error? return it; the old pipeline is untouched
    device_wait_idle                // frames in flight may still use the old one (§ 19)
    destroy old, put new in slot id // same id: the app's tickets stay valid
```

### 2. scop: watch, compile, swap

```
ShaderWatch { folder, last_seen: (mtime vert, mtime frag), since_check }
    folder = scop/shaders, as written into the binary at build time (CARGO_MANIFEST_DIR)
    missing folder or no glslc → print "hot reload off: …" once, never check again
update: since_check += dt; ≥ 0.25 s → read both mtimes
        newer than last_seen → remember them, then:
            glslc mesh.vert -o -  and  glslc mesh.frag -o -    (SPIR-V on stdout)
            error → print glslc's message (file:line: error …), keep the old pipelines
            ok    → replace_pipeline for every style scop has (fill, wireframe, points)
                    print "shaders reloaded"
```

The built-in shaders (`include_bytes!`) are still what scop starts with; the committed `.spv`
fallback is untouched — `make spv` still refreshes it.

### Decisions

| Choice | Over | Why |
|---|---|---|
| **Poll modification times every 0.25 s** | A file-watching library (`notify`) | Only window/event libraries are allowed; `std::fs::metadata` is enough |
| **Run `glslc` at runtime** | Shipping a GLSL compiler inside scop | Same tool as the build; nothing new to install or write |
| **New pipelines first, old kept on failure** | Destroy then build | Same rule as § 19: a broken shader never leaves scop without one |
| **Replace in the same slot (id kept)** | New id, app updates its tickets | The id means "this pipeline"; the app shouldn't care about versions (Q-21.5) |
| **Source folder baked in at build time** | A command-line flag | Works from the repo with no setup; anywhere else, reload just switches off |
| **Hot reload in every build** | Debug builds only | Harmless when the folder or `glslc` is missing; handy in release for checking frame rates |

---

## Known traps

- **Reload error mid-save** — some editors write the file in two steps; scop catches it half-written
  and `glslc` complains. The next save (or the next check) fixes it — the old shaders run meanwhile.
- **Nothing happens on save** — the editor saved somewhere else (a copy, another folder), or hot reload
  switched itself off at start: read the first lines scop printed.
- **New input, compiles fine, but validation complains** — the shader contract changed (Q-21.4). Undo
  it, or make the Rust side match and restart.
- **Validation error destroying the old pipeline** — destroyed while a frame in flight used it: wait
  first (§ 19, Q-19.3).
- **Committed `.spv` out of date** — hot reload never writes it; `make spv` before committing (§ 16).
- **`glslc` found at build time but not at runtime** — the program's search path differs (e.g. started
  from an app launcher). scop says so and carries on.

---

## Checkpoint

**Satisfies (subject):** bonus extra (shader hot reload). Method: gate first, then Claude writes it
and walks you through it.

1. **Engine:** `Shaders<'a>`; `Gfx::replace_pipeline` (new first, wait, destroy old, same slot).
2. **scop:** `ShaderWatch` (folder, mtimes, 0.25 s polling, switches off cleanly); `compile` with
   `glslc -o -`; reload every style's pipeline; messages for reloaded / error / off; README.
3. **Tests:** a newer mtime is noticed exactly once; no change = no reload; a missing folder switches
   the watch off; `compile` returns `glslc`'s error text for a broken shader (skipped when `glslc` isn't
   installed).

**Verify:** run scop, change `FRONT` in `mesh.frag` to green, save — facing mode turns green without
a restart; add a typo, save — error printed, picture unchanged; fix it — reloaded; validation silent.

---

## Questions

Q-21.1 (recall) Why does scop have to run `glslc` before it can use the edited shader — why can't
it hand the `.frag` file to Vulkan directly?

Q-21.2 (application) Why build the new pipelines *before* destroying the old ones?

Q-21.3 (prediction) You save `mesh.frag` with a missing semicolon. What do you see on screen, and what
in the terminal? What happens when you fix it and save again?

Q-21.4 (prediction) You add `layout(location = 4) in vec3 in_tangent;` to `mesh.vert` and save. It
compiles. Why does it still not work, and what does it take to make it work?

Q-21.5 (application) § 19 never reuses a destroyed mesh's slot, but § 21 puts the new pipeline in
the old one's slot, keeping the same id. Why is reusing the slot fine here?

Answers: `ANSWERS.md` § 21
