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
