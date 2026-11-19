---
section: 20
slug: bonus-camera-and-lighting
tier: T1
status: done
prereqs: [10, 13, 18, 19]
terms: [orbit camera, yaw and pitch, pitch clamp, look-at, mouse delta, directional light, normal, flat shading, diffuse, ambient, world-fixed light, normal transform]
---

# Section 20 — A camera you steer, and light (bonus)

**Depth: T1, no maths (owner's choice, 2026-11-19).** Planned as T2; lowered because the owner
has built orbit cameras and lighting before (rubik, cubed). This section says **what each piece
does, where it lives, and how to see that it's set up properly**. The formulas are in the
code, with tests proving them; they aren't derived here.
**You'll be able to:** orbit around the model with the mouse, light it with one sun-like light,
and tell from the screen whether the camera, the normals and the light are wired correctly.

---

## The idea

**Two ways to turn things.** Until now the *model* turned (keys, auto-spin) and the camera
stood still. An **orbit camera** does the opposite: the model stays, the camera walks around
it on a sphere, always looking at the model's centre. Drag with the mouse: sideways walks
round (**yaw**), up/down climbs over or under (**pitch**). The keys still turn the model, so
both work together.

On screen the two look identical — *until there's light*. A light fixed in the world doesn't
care where the camera stands, so orbiting shows you the lit side and the dark side in turn.
Turning the model with the keys changes *which faces* are lit. That difference is the easiest
check that both are set up properly (Q-20.1).

**Pitch has a limit.** Climb straight over the top and "up" stops meaning anything — the
camera flips upside down in one frame. So pitch stops just short of straight up and down.

**One light, like the sun.** A **directional light** comes from one direction everywhere, from
infinitely far away. A face's brightness depends on how directly it faces the light: facing
it = full brightness, side-on = dim, facing away = only the **ambient** glow (a small constant
so nothing goes pure black against the background).

**Normals.** "Which way a face points" — you've had it since § 17 (Newell) and § 18 (box
mapping). Lighting needs it *per corner*, sent to the shader like the UVs. Because every
triangle already has its own corners (§ 13), each face's corners can carry that face's own
normal: **flat shading** — each face one even brightness, edges crisp. Exactly right for the
logo; the teapot will look faceted, which is honest for its file.

**Normals turn with the model.** When the model turns, its faces point somewhere new, so their
normals must turn too — the same turn as the model matrix, without the move. Forget it and the
light looks painted on: the bright faces stay bright however the model spins (Q-20.3).

**Where the analogy stops being true:** a real sun casts shadows. This light doesn't — a face
pointing at the light is lit even if another part of the model is in the way. Shadows are a
separate technique (ft_vox / later).

---

## The names for it

**orbit camera** — a camera on a sphere around a target, always looking at it. Controlled by yaw,
pitch and distance.

**yaw and pitch** — yaw: walk round the target (left/right). Pitch: climb over / under (up/down).

**pitch clamp** — stopping pitch a little before straight up or down (±89°), so the camera never
flips.

**look-at** — the engine's ready-made view card: "stand here, look at that, this way is up". scop
uses it instead of the fixed `translation(0, 0, −distance)`.

**mouse delta** — how far the mouse moved this frame, in pixels. The camera turns by the *movement*,
not by where the cursor is.

**directional light** — light from one direction everywhere (a sun). Only a direction, no position.

**normal** — the direction a face points, length 1. One per corner, sent to the shaders.

**flat shading** — every corner of a face carries the face's normal: one brightness per face.

**diffuse** — the brightness from facing the light: full when facing it, zero when side-on or away.

**ambient** — a small constant brightness added everywhere, so faces turned away aren't black.

**world-fixed light** — the light's direction belongs to the world, not the camera: orbiting doesn't
move it. (The other choice, a "headlight" fixed to the camera, always lights what you look at.)

**normal transform** — turning the normals by the model's rotation (the model matrix without its
move). scop only turns and moves models, never stretches them, so that's all it needs.

---

## How it actually works

### 1. Mouse input (engine)

```
winit CursorMoved / MouseInput → Input::mouse_held(MouseButton::Left)
                                 Input::mouse_delta() → (dx, dy) in pixels this frame
reset at end_frame (like scroll); the first move after a press doesn't jump
```

### 2. Orbit camera (engine maths + scop)

```
engine: Mat4::look_at(eye, target, up)          // given + tested, not derived
scop:   Orbit { yaw, pitch }                    // distance = the § 12 framing distance
        dragging: yaw −= dx × 0.005, pitch += dy × 0.005 (radians per pixel)
        pitch clamped to ±89°
        eye = a point on the sphere around the origin (yaw, pitch, distance)
        view = look_at(eye, origin, up = +y)    // the model is centred on the origin already
        R resets the orbit too
```

### 3. Normals and light (engine vertex + uniform, scop shaders)

```
engine Vertex: + normal [f32; 3]   (location 3)       // § 7's packing list grows
engine camera uniform: view_proj + light (vec4)        // § 10's noticeboard grows: 64 → 80 bytes
        Frame::set_light(direction, strength)          // strength 0 = lighting off
scop:   corner normal = its face's normal (§ 18's face_normals), made length 1
mesh.vert: normal_out = rotation part of push.model × normal     // normals turn with the model
mesh.frag: brightness = ambient + strength × max(0, facing-the-light)
           normal mode: colour × brightness;  facing / depth modes: unlit (they're diagnostics)
        B toggles the light (strength 1 ↔ 0)
```

### Decisions

| Choice | Over | Why |
|---|---|---|
| **Orbit camera on mouse drag**, keys still turn the model | Replacing the key controls | The subject's controls stay; the camera is an extra |
| **Distance = framing distance**, wheel stays zoom (§ 18) | Wheel moves the camera in/out | Zoom is already there; W/S already dolly the model |
| **Pitch clamp ±89°** | Free rotation (quaternions) | Never flips; quaternions are ft_newton material |
| **`look_at` in the engine** | Building the view from rotations in scop | Every later project needs it |
| **One directional, world-fixed light** | Headlight / point lights | Orbiting then *shows* the shading change (Q-20.1); one light is the scope |
| **Light in the camera uniform** | More push params | Push params are full (§ 19), and the light is per frame, not per draw (§ 10's noticeboard vs sticky note) |
| **Flat normals per corner** | Smooth (averaged) normals | Corners are already per face (§ 13); right for the logo; smooth is an easy later change |
| **Normals turned by the model's rotation only** | The general inverse-transpose | scop never stretches models; noted as a trap for later projects |
| **Ambient 0.25** | No ambient | Faces turned away stay visible against the dark background |

---

## Known traps

- **Camera flips over the top** — pitch not clamped (Q-20.2).
- **Camera jumps when you start dragging** — the delta measured from an old cursor position;
  the first move after a press must start from where the cursor is now.
- **Light looks painted on** — normals not turned with the model (Q-20.3).
- **Model all black, or lit from the wrong side** — the light direction pointing the wrong way
  (towards the light vs from it), or normals pointing inwards. Facing mode (§ 19) checks the
  faces; a light from behind the camera should light what you see.
- **Validation error on the vertex input** — `Vertex` grew but the shader's location 3 or the
  attribute list didn't (§ 7).
- **Uniform reads garbage** — the camera block grew in the shader but not the buffer (or the
  reverse): 64 → 80 bytes on both sides (§ 10).
- **Teapot looks faceted** — expected with flat shading, not a bug.
- **Stretched model lit wrongly** (later projects) — non-uniform scale needs the inverse-transpose
  for normals. scop never stretches, so it doesn't apply here.

---

## Checkpoint

**Satisfies (subject):** bonus extras (orbit camera, directional light). Method: gate first, then
Claude writes it and walks you through it.

1. **Engine:** `Input::mouse_held`, `Input::mouse_delta`; `Mat4::look_at` (tested); `Vertex.normal`
   (location 3); camera uniform + light vec4, `Frame::set_light`.
2. **scop:** flat normals from the face normals; `Orbit` with drag, clamp and R reset; world-fixed
   light from above-front-left; B toggles; lighting only in normal colour mode; controls and README.
3. **Tests:** `look_at` puts the target straight ahead and keeps up as up; the pitch clamp holds;
   normals have length 1 and point outwards on the cube; mouse delta resets each frame.

**Verify:** drag round the logo — the lit side swings into and out of view while the faces' shading
stays put on the model; J/L instead — which faces are lit changes; drag hard over the top — no flip;
B off/on; facing and depth modes unlit; validation silent.

---

## Questions

Q-20.1 (application) Turning the model with J and orbiting the camera with the mouse look the same
without light. With the world-fixed light on, what looks different between the two?

Q-20.2 (prediction) Pitch isn't clamped and you drag upwards past straight-up. What do you see?

Q-20.3 (prediction) The normals reach the shader but aren't turned by the model. You spin the model
with J / L. What does the lighting do?

Q-20.4 (application) Why add an ambient amount instead of using only how directly a face faces the
light?

Q-20.5 (application) Why does flat shading rely on each triangle having its own corners (§ 13)? What
would shared corners give instead?

Answers: `ANSWERS.md` § 20
