---
section: 16
slug: mandatory-finish
tier: T1
status: in-progress
prereqs: [15]
terms: [swapchain recreation, out of date and suboptimal, size-dependent objects, dependency check, Makefile target, release build, self-evaluation, defence]
---

# Section 16 — Making the mandatory part perfect

**Depth: T1** — no new concepts: this is the finishing pass. Three gaps remain between
"it works on my Mac" and "it passes evaluation": the window can't be resized cleanly,
there's no one-line setup command, and the README is still placeholders. Then you check
everything against the subject, because the bonus is only graded if the mandatory part is
**perfect**.
**You'll be able to:** resize the window freely, hand anyone a fresh clone that builds with
`make`, and walk an evaluator through every requirement with evidence.

---

## The idea

**Resizing.** The swapchain (§ 4) is a set of images made for *one* window size. Drag the
window bigger and those images no longer fit; Vulkan tells you so the next time you
acquire or present ("out of date"), and so far scop just skipped those frames. The fix
is to rebuild the swapchain at the new size — and the depth image with it, since it must
match (Q-11's explain-back). Everything else stays: the pipelines set viewport and scissor
per frame (§ 6), the camera reads `aspect` every frame (§ 10), the meshes and textures
don't care about the window at all.

Rebuilding is like changing the canvas under a painter: wait until the painter has put the
brush down (the GPU is idle), swap the canvas (and its matching depth sheet), carry on.

**One-line setup.** The subject: *"Provide a one-line command to check for and possibly
install all required dependencies."* For scop that's `make`: check that Rust, the shader
compiler (`glslc`) and Vulkan are there, install what can be installed, then build.

**README.** The subject lists exactly what it must contain — and one item matters more than
it looks: *"a description of how AI was used — specifying for which tasks and which parts
of the project."* A lot of scop's code was written by Claude after your gates, so this
must be honest and specific. At the defence, the evaluator will check that you can explain
those parts — which is exactly what every explain-back has been for.

**Self-evaluation.** Go through the subject line by line and, for each requirement, note
*how to show it works*. Include things evaluators always try: their own `.obj` files,
resizing, minimising, closing with Esc and the window button.

**Where the analogy stops being true:** a painter could keep going on the old canvas;
Vulkan won't — an out-of-date swapchain can't be presented to at all.

---

## The names for it

**swapchain recreation** — destroying the old swapchain (images, views, `render_done`
semaphores) and making a new one at the window's current size, with the same format and
present mode.

**out of date and suboptimal** — what acquire / present report after a resize:
`ERROR_OUT_OF_DATE_KHR` = can't use this swapchain any more; `SUBOPTIMAL_KHR` = still
works, but no longer matches the window exactly (e.g. scaled). scop rebuilds on both.

**size-dependent objects** — everything sized to the window and rebuilt with the
swapchain: its images and views, and the depth image. Not: pipelines (dynamic viewport),
camera (aspect read per frame), meshes, textures.

**dependency check** — a script that looks for each required tool (`cargo`, `glslc`, the
Vulkan loader), reports what's missing, and installs what it safely can.

**Makefile target** — a named recipe in a `Makefile`: `make` (deps + build), `make run`,
`make clean`. `make` with no name runs the first one.

**release build** — `cargo build --release`: optimised, and (in scop) without the
validation layer, which slows things down and is a development tool.

**self-evaluation** — checking every subject requirement yourself, with a way to show it,
before anyone else does.

**defence** — the evaluation conversation: the evaluator runs scop, loads their own
files, and asks you to explain any part of the code and any decision.

---

## How it actually works

### 1. Resize

```
platform:  WindowEvent::Resized(size)  → remember "needs rebuild" (unless 0×0: minimised)
           draw_frame returns OutOfDate or Suboptimal → also "needs rebuild"
before the next frame, if "needs rebuild":
    device_wait_idle                               // the GPU puts the brush down
    destroy depth image, then swapchain (views, render_done semaphores, swapchain)
    create swapchain at window.inner_size()        // same choose_* functions as § 4
    create depth image at the new extent           // same DepthImage::new as § 11
```

`device_wait_idle` is the simple, always-correct choice: resizes are rare, so pausing the
GPU for a moment costs nothing anyone notices. The `image_free` semaphores and fences
belong to the frame slots (§ 5), not to the swapchain, so they survive.

### 2. `make`

```
Makefile
  all:   deps build          ← `make`
  deps:  scripts/deps.sh     ← check + install
  build: cargo build --release
  run:   cargo run --release -p scop -- $(OBJ)      (OBJ ?= resources/42.obj)
  clean / fclean / re        ← the usual 42 names

scripts/deps.sh
  cargo missing   → install Rust with rustup into your home folder (no admin needed)
  glslc missing   → macOS: brew install shaderc          Linux: offer to curl the LunarG SDK
                    into your home folder (no admin); the build still works meanwhile,
                    from the committed .spv fallback (decision: option C)
  Vulkan missing  → macOS: brew install molten-vk vulkan-loader   Linux: print the command
  everything found → "all dependencies OK"
```

On a campus Linux machine you can't install system packages without admin rights, so the
script checks and *tells you* rather than failing silently — "possibly install", as the
subject says.

### 3. README

Exactly the subject's sections, plus two that evaluators will look for:

```
*This project has been created as part of the 42 curriculum by luferna3.*
# scop
## Description        — what it is, what it shows, the engine/app split
## Instructions       — make, make run OBJ=…, the controls table, swapping the texture
## Features / technical choices  — short, pointing at DECISIONS.md
## Resources          — Vulkan spec & tutorial, ash, winit, OBJ & TGA specs, sRGB, …
### AI usage          — learning plan and teaching material; checkpoint code per section,
                        written by Claude after each gate and explained back; what you
                        decided, tested and chose yourself
```

The AI section is yours to sign: Claude drafts it from `PROGRESS.md` and the checkpoints;
you edit it until every sentence is true in your own words.

### 4. Self-evaluation checklist

| Subject requirement | How to show it |
|---|---|
| Parses `.obj` and renders it | `make run`, then an evaluator's file |
| Perspective | far parts smaller; push away with S |
| Rotates around its centre, 3 axes; translates | spin + I K / J L / U O; arrows, W S |
| Faces distinguishable, subtle greys | flat greys at start |
| Key applies texture, again toggles back, smooth | T, and T again mid-fade |
| Cheerful texture | your picture |
| No external libs for obj / maths / shaders (/ images) | `Cargo.toml`: only winit, raw-window-handle, ash, ash-window; `DECISIONS.md` |
| One-line dependency command | `make` on a fresh clone |
| README format | first line, the three sections, AI usage |
| Robust | other `.obj` files (quads, n-gons, `vt`/`vn`, `1/2/3`, negative indices), a broken file, resize, minimise, Esc, close button — validation silent throughout |

---

## Known traps

- **Validation error / crash on resize** — the swapchain destroyed while the GPU still uses
  it (no `device_wait_idle`) (Q-16.2).
- **Picture stretched after a resize** — swapchain rebuilt but `aspect` cached somewhere,
  or the depth image left at the old size (validation error about render area).
- **Hang after minimising** — rebuilding to a 0×0 size; skip until the window is real again.
- **`make` works only on your Mac** — a path or tool that only exists there (Homebrew
  prefix, a hard-coded `/Users/...`). Test on a fresh clone; on campus if you can.
- **`make` silently does nothing useful** — a missing tool reported as success. Every
  check must say clearly what's missing and how to get it.
- **README AI section too vague** ("AI helped a bit") — the subject asks *which tasks* and
  *which parts*. Vague reads as hiding.
- **An evaluator's `.obj` crashes scop** — anything that panics instead of an error message.

---

## Checkpoint

**Satisfies (subject):** the technical constraints (one-line command), the README chapter,
and "mandatory part perfect" so the bonus counts. Method: gate first, then Claude writes it
and walks you through it.

1. **Engine:** swapchain + depth rebuild on `Resized` / out-of-date / suboptimal, skipping
   0×0; validation silent while dragging the window.
2. **`Makefile` + `scripts/deps.sh`** as above.
3. **README** drafted in full; **you** edit the AI usage section until it's yours.
4. **Self-evaluation:** run through the checklist; two extra test `.obj` files in
   `resources/test/` (one with `vt`/`vn` and `1/2/3` tokens, one with an n-gon and negative
   indices) and one broken file.

**Verify:** the whole checklist above, on your Mac — and `make` on a fresh clone.

---

## Questions

Q-16.1 (recall) Why does resizing the window need a new swapchain? Which other object must be
rebuilt with it, and why do the pipelines *not* need rebuilding?

Q-16.2 (prediction) After an out-of-date result you destroy and recreate the swapchain
straight away, without `device_wait_idle`. What can go wrong?

Q-16.3 (application) What does `make` check for, and what can't it install on a campus Linux
machine? Why is that still within "check for and possibly install"?

Q-16.4 (recall) What four things must the README contain, according to the subject?

Q-16.5 (application) An evaluator loads their own `.obj` with `vt` and `vn` lines, faces
written `f 1/1/1 2/2/2 3/3/3`, and one 5-corner face. What does scop do with each of those?

Answers: `ANSWERS.md` § 16
