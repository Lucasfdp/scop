---
section: 24
slug: engine-consolidation
tier: T1
status: done
prereqs: [8, 18, 19, 20, 21, 22, 23]
terms: [consolidation, reuse test, default shaders, shader override, hard-coded vs setting, mesh builder, colour callback, tween]
---

# Section 24 — Moving the bonus features into the engine

**Depth: T1, no maths.** Almost nothing new to learn: this section *moves* code. The skill is
deciding **where each piece belongs**, and making the engine version general enough that the
next project can use it unchanged. Owner's request, 2026-11-21: "everything reusable from the
bonus goes into the engine", like § 8 did for the mandatory part.
**You'll be able to:** sort any feature into engine or app with two questions, spot the
assumptions that would stop it being reused, and leave scop as "the subject's choices only".

---

## The idea

**Two questions, the same as § 8.** For every piece of scop:

1. **Would ft_newton, ft_vox or the Rust engine want this, unchanged?** → engine.
2. **Is it a choice the subject (or scop) made?** → scop.

An orbit camera passes test 1: every 3D project wants one. The grey faces fail it: they exist
because the subject asks for distinguishable greys — that's scop's choice (Q-24.1).

**Unchanged is the hard part.** Code written for scop quietly assumes scop's world. The orbit
camera assumes "up" is +y; ft_newton's world might have +z up. Moving it into the engine means
turning each assumption into a **setting** with scop's value as the default, not copying it
as it is (Q-24.5).

**Default shaders.** Lighting, chrome, the debug colours and the sky live in scop's shader
files today. Owner's decision: the engine ships them as **defaults**. An app gets them by
asking for a default pipeline; an app that wants something else still writes its own shaders
(an **override**) and draws with `draw_with`, exactly as today. The default shaders' push
params get *named* meanings (blend, colour mode, depth range) in a small engine type, so apps
stop packing four bare numbers by hand (Q-24.3).

**Splitting a feature down the middle.** Some features are half general, half scop. The
`.obj` → mesh step is general (split objects, flat normals, box-mapped UVs); the grey per face
is scop's. So the engine's **mesh builder** takes the colour as a **callback**: "for face *n*,
what colour?" — scop passes its greys, ft_newton passes its own. Hot reload splits the same
way: the engine notices saves and compiles; the app decides which of *its* pipelines get the
new shaders (Q-24.4).

**What scop keeps:** key bindings and the controls text, the grey-face rule, which picture each
object gets (N), the 8 × 8 × 8 crowd layout, the window title's wording, `main()`.

**Where the analogy stops being true:** "move it to the engine" sounds like cut and paste. Most
of the work is the opposite — deciding what *not* to move, and which values become settings.

---

## The names for it

**consolidation** — gathering scattered, reusable code into the one place every project uses.

**reuse test** — the two questions above: wanted unchanged elsewhere? a choice this app made?

**default shaders** — shaders the engine ships and compiles itself (lighting, chrome, debug colours,
sky), used through `Gfx::create_default_pipeline` and friends.

**shader override** — an app's own shaders used instead of the defaults; the engine doesn't care which.

**hard-coded vs setting** — a value written into the code (scop's +y up) vs one the caller chooses,
with a sensible default.

**mesh builder** — the engine's `.obj` → meshes step: one mesh per object, own corners per triangle,
flat normals, optional box-mapped UVs, colour from a callback.

**colour callback** — a function the app hands the engine ("face n → colour"), so the engine stays
general and the app keeps its rule. In Rust: a closure, `|face| …`.

**tween** — a value that eases towards a target over time (the § 15 fade, generalised).

---

## How it actually works

### What moves, and what it becomes

| Today (scop) | Engine, after | Setting instead of hard-coded |
|---|---|---|
| `Orbit`, zoom, framing distance, near/far, `depth_range` | `camera.rs`: `OrbitCamera` | up axis, pitch limit, zoom limits, sensitivity |
| `Placement` (+ its matrix around a centre) | `transform.rs`: `Transform` | — |
| `Fade` + `smoothstep` | `tween.rs`: `Tween` | duration, easing |
| `object_meshes`, `face_normals`, `box_uv`, `main_axis` | `mesh_builder.rs` | UV mapping (none / box), colour callback |
| `hot_reload.rs` | `shader_watch.rs` | the list of files |
| `sky.rs`, `sky_box`, `load_sky` | `sky.rs` | sun direction, colours, face size |
| `mesh.vert/frag`, `sky.vert/frag` | `engine/shaders/` (compiled by the engine's own `build.rs`) | push params named by `DefaultParams` |
| `load_model` (read + parse + build) | `mesh_builder::load_obj` | colour callback, UV mapping |

### What the default shaders need

```
engine/build.rs        compiles engine/shaders/*.vert|frag (and the committed .spv fallback, § 16)
Gfx::create_default_pipeline(style)       // fill / wireframe / points, default shaders
Gfx::create_default_sky()                 // background pipeline + box + generated cube
DefaultParams { blend, colour_mode, depth_range } → the four push numbers
ColourMode { Normal, Facing, Depth, Chrome }
hot reload of the default shaders: the engine watches engine/shaders/
```

### Decisions

| Choice | Over | Why |
|---|---|---|
| **Settings with scop's values as defaults** | Copying scop's assumptions | Reuse means unchanged *use*, not unchanged *values* |
| **Colour as a callback** | Greys in the engine / no colours | The rule stays scop's; the mechanism is general |
| **Default shaders + overrides** | Every app writes shaders / engine forces its shaders | Projects start working at once; any can still replace them |
| **`DefaultParams` names the push numbers** | Bare `[f32; 4]` | Only for the default shaders; custom shaders keep raw `draw_with` |
| **Engine notices + compiles, app picks pipelines** | Engine rebuilds pipelines itself | Only the app knows which of its pipelines use which shaders |
| **Crowd layout stays in scop** | A grid helper in the engine | Every project places copies its own way (owner, 2026-11-21) |
| **Tests move with their code** | Rewriting tests | They prove nothing changed in the move |

---

## Known traps

- **Behaviour changes in the move** — a default that isn't scop's old value (pitch limit, zoom steps).
  The tests moving with the code catch most; then check by eye.
- **The engine starts knowing scop** — a constant like `LIGHT_DIRECTION` or the grey rule ending up in
  the engine. Run the reuse test on every constant.
- **Two copies of a shader** — scop's and the engine's drift apart. After the move, scop has no shader
  files of its own (it uses the defaults).
- **Hot reload watches the wrong folder** — it must watch `engine/shaders/` now.
- **Push params meaning two things** — the default shaders own their meaning; custom shaders use raw
  numbers. Never mix the two in one pipeline.

---

## Checkpoint

**Satisfies (subject):** nothing new — the same scop, cleaner inside. Method: gate first, then Claude
writes it and walks you through it.

1. **Engine:** `camera.rs`, `transform.rs`, `tween.rs`, `mesh_builder.rs`, `shader_watch.rs`, `sky.rs`;
   `engine/shaders/` + `engine/build.rs`; `create_default_pipeline`, `create_default_sky`,
   `DefaultParams`, `ColourMode`. Every new public item documented, nothing scop-shaped inside.
2. **scop:** uses only the engine's public API for all of it; keeps keys, greys, picture rotation,
   crowd layout, title, `main()`. No shader files of its own.
3. **Tests:** every moved test passes in its new home; a test using a non-default setting (e.g. +z up)
   for the camera.

**Verify:** scop behaves exactly as at the end of § 23 — every key, crowd, chrome, sky, hot reload
(now on `engine/shaders/`); validation silent; `main.rs` much shorter.

---

## Questions

Q-24.1 (application) Run the two reuse questions on `face_grey`. Engine or scop — and what part of
building coloured meshes *does* go to the engine?

Q-24.2 (application) Why should the engine's `OrbitCamera` own the pitch clamp, instead of leaving each
app to remember it?

Q-24.3 (application) The default shaders read push params as blend, colour mode and depth range. Your
ft_newton shader wants those four numbers for something else. What do you do, and why not just reuse
the default pipeline?

Q-24.4 (application) Hot reload is split: the engine notices saves and compiles, the app decides which
pipelines to rebuild. Why can't the engine do the second part on its own?

Q-24.5 (prediction) ft_newton's world has +z as up. You move scop's `Orbit` into the engine exactly as
it is. What goes wrong when ft_newton uses it, and what should have been a setting?

Answers: `ANSWERS.md` § 24
