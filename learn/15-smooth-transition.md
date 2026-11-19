---
section: 15
slug: smooth-transition
tier: T1
status: done
prereqs: [14]
terms: [blend factor, linear interpolation, target value, transition duration, easing, smoothstep, reversible transition, push constant range for two stages]
---

# Section 15 — Fading colour into texture

**Depth: T1** — one number, one key, one line of shader. The ideas behind it
(animate towards a target, ease in and out) are everywhere in games and UI, so they're
worth having clearly, but there's no new GPU machinery.
**You'll be able to:** press T to fade smoothly from the grey faces to the picture, press
it again (even halfway) to fade back, with no jump anywhere.

---

## The idea

The subject: *"pressing a dedicated key should apply a texture to the object. Pressing
that same key again should toggle it back to the colored view. A smooth transition
between the two is expected, with no abrupt cuts."*

Right now each pixel has two possible colours: its face's grey (§ 13) and its spot on the
picture (§ 14). A transition is just **a mix of the two**, controlled by one number from 0
to 1: at 0 all grey, at 1 all picture, at 0.3 "70 % grey, 30 % picture". Slide that number
from 0 to 1 over half a second and the picture fades in.

Think of a dimmer switch rather than a light switch. The key doesn't *set* the light; it
says which way to turn the dimmer. Every frame, the dimmer moves a small step towards
where it's been told to go. Press the key again halfway, and the dimmer simply starts
turning back from wherever it is — no jump, because nothing ever teleports the knob.

One more touch: a dimmer turned at a constant speed starts and stops abruptly. Easing it
— slow at the start, quick in the middle, slow at the end — is what makes a fade feel
smooth rather than mechanical.

**Where the analogy stops being true:** a dimmer has one knob for the whole room. Here the
number is sent with each draw (the sticky note again), so in a scene with several models
each could fade on its own.

---

## The names for it

**blend factor** — the 0..1 number saying how much of the second colour to use. Here:
how much picture (vs grey).

**linear interpolation** — `a + (b − a) × t`: the point a fraction `t` of the way from `a`
to `b`. Often called **lerp**; in GLSL the built-in is `mix(a, b, t)`.

**target value** — where the blend factor is heading: 0 (grey) or 1 (picture). The T key
flips the target, never the blend factor itself.

**transition duration** — how long a full fade takes. scop: 0.6 s. The blend factor moves
by `dt / duration` per frame, clamped to 0..1.

**easing** — reshaping the 0..1 progress so motion starts and ends gently instead of
abruptly. The input is the raw progress; the output is what's actually used.

**smoothstep** — the classic easing curve: `t² × (3 − 2t)`. At 0 and 1 its slope is zero,
so the fade starts and stops without a "kick". GLSL has it built in; scop applies it on
the CPU.

**reversible transition** — one that can change direction at any moment without a jump.
Comes for free from "move the value towards a target" instead of "restart an animation".

**push constant range for two stages** — the § 10 range grows from "vertex shader, 64
bytes" to "vertex **and fragment** shaders, 68 bytes": the model matrix plus one float.
Both shaders declare the same block; each uses its own part.

---

## How it actually works

### 1. State and keys (scop)

```
state:   texture_target: f32   (0 or 1)
         texture_progress: f32 (0..1, raw, linear)

update:  if pressed(T): texture_target = 1 − texture_target       (pressed: a toggle, Q-13.3)
         step = dt / FADE_SECONDS
         move texture_progress towards texture_target by at most `step`
```

"Towards by at most `step`" means: if it's below the target, add `step` but don't pass
the target; if above, subtract. When it arrives, it stays.

### 2. Easing and sending (scop → engine)

```
draw:    blend = smoothstep(texture_progress)       // t² × (3 − 2t)
         frame.draw_textured(pipeline, mesh, texture, model, blend)
```

### 3. The sticky note grows (engine)

```
push constant range:  stages VERTEX | FRAGMENT, offset 0, size 68
                      bytes 0..64 = model (mat4), bytes 64..68 = blend (float)
cmd_push_constants:   same stages, the 68 bytes
```

### 4. The shaders

```glsl
// both mesh.vert and mesh.frag
layout(push_constant) uniform Push {
    mat4 model;
    float blend;   // § 15: 0 = grey, 1 = picture
} push;

// mesh.frag
out_colour = mix(vec4(colour, 1.0), texture(picture, uv), push.blend);
```

The mix happens on real brightness, not encoded numbers, because the texture is `SRGB`
(decoded on read) and the swapchain is `SRGB` (encoded on write) — § 14's footnote: a
50 % mix looks like halfway.

### Decision: CPU-side animation, blend in the push constant, smoothstep

| Choice | Over | Why |
|---|---|---|
| **Animate on the CPU with `dt`, towards a target** | A timer started on key press (restarting jumps; harder to reverse mid-way) | Reversible for free; same per-second idea as § 13 |
| **Blend factor in the push constant** | In the camera uniform buffer | It's per *draw* (each model could fade on its own), not per frame |
| **smoothstep easing** | Straight linear fade | No "kick" at start and end — the subject's "no abrupt cuts" |

---

## Known traps

- **The picture snaps in / out** — the key sets the blend factor directly instead of the
  target; or `pressed` vs `held` swapped (holding flips it every frame).
- **A jump when pressing T halfway** — the fade restarts from 0 or 1 instead of moving
  from where it is.
- **Fade speed depends on the machine** — the step isn't `dt / duration`.
- **Validation error at pipeline creation: push constant range** — the fragment shader
  reads push constants but the range's stage flags only say `VERTEX` (Q-15.4).
- **Validation error at `cmd_push_constants`** — its stage flags or size don't match the
  range exactly.
- **Blend stuck slightly off 0 or 1 / overshoots** — no clamp when stepping towards the
  target.
- **Halfway looks too dark or too light** — mixing encoded colours (a `UNORM` texture);
  keep both `SRGB`.

---

## Checkpoint

**Satisfies (subject):** "a dedicated key applies the texture and toggles back … smooth
transition, no abrupt cuts" — the last mandatory feature. Method: gate first, then Claude
writes it and walks you through it.

1. **Engine:** push range `VERTEX | FRAGMENT`, 68 bytes; `Draw` carries the blend factor;
   `Frame::draw_textured(…, blend)`; `Frame::draw` uses blend 0 (colours only).
2. **Shaders:** the shared push block in both; `mix` in the fragment shader.
3. **scop:** target + progress, T toggles the target, `dt / FADE_SECONDS` stepping with a
   clamp, smoothstep, T added to the printed controls. Tests: stepping reaches and stays at
   the target, reverses mid-way without a jump; smoothstep's ends and middle.

**Verify:**
- `cargo run -p scop -- resources/42.obj`: starts grey; T fades to the picture in ~0.6 s;
  T fades back; T pressed mid-fade reverses smoothly.
- Validation silent; `cargo test`, `cargo clippy --all-targets` clean.

---

## Questions

Q-15.1 (recall) What does `mix(a, b, t)` give at `t = 0`, `t = 0.5` and `t = 1`? In scop,
what are `a`, `b` and `t`?

Q-15.2 (application) Why does the T key flip a *target* instead of setting the blend
factor directly to 0 or 1?

Q-15.3 (prediction) You press T to fade to the picture, then press it again when the
fade is 40 % done. Describe what happens over the next half second.

Q-15.4 (prediction) You add `float blend` to the fragment shader's push block, but leave
the push constant range's stages as `VERTEX` only. What happens, and when?

Q-15.5 (application) smoothstep gives the same start (0) and end (1) as a straight
linear fade. So what's the visible difference, and where in the fade do you see it?

Answers: `ANSWERS.md` § 15
