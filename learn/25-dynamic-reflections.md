---
section: 25
slug: dynamic-reflections
tier: T1
status: done
prereqs: [5, 11, 23, 24]
terms: [render to texture, render target, reflection probe, cube face camera, probe exclusion, offscreen pass, layout hand-over]
---

# Section 25 — Live reflections (bonus)

**Depth: T1, no maths (as §§ 20–24).** Owner's request (2026-11-21): chrome should reflect
**whatever is actually around it** — the background colour, the sky if it's on, anything else
in the scene — not a picture chosen in advance. Built straight into the engine (§ 24 rule).
**You'll be able to:** say how a renderer draws into a texture instead of the screen, why a
live reflection is six extra views of the scene, and what that costs.
**Also in this checkpoint (owner's request, no learning section):** the controls written on
screen, toggled with H.

---

## The idea

**Drawing into a picture.** Until now every frame was drawn into a swapchain image — the
screen. Nothing stops the GPU from drawing into an ordinary image instead: a **render
target**. Same pipelines, same draws, same depth test; the result just isn't shown. It's a
picture you made, and a shader can read it afterwards like any texture (§ 14). This is
**render to texture**, and it's the door to most "advanced" effects: reflections, shadows,
mirrors, security-camera screens, post-processing.

**A live cube map.** § 23's cube map was six pictures made once at start. A live one is six
pictures **made every frame**: put a camera at the model's centre, point it at each of the six
walls in turn (right, left, up, down, front, back), and draw the scene each time with a square,
90° view — six views that together see in every direction. That spot is a **reflection
probe**. Chrome then reads this cube instead of the fixed sky: green background → green
reflection; sky on → sky; anything else in the scene → in the reflection (Q-25.3).

**Leave the mirror out of its own photo.** The probe sits *inside* the model. Drawing the model
into its own cube would fill every face with the model's insides, and chrome would reflect
that. So the reflective draws are **excluded** from the probe's views (Q-25.1) — the same
honest limit as § 23: the model never reflects itself.

**Order matters.** The six probe views must be finished before chrome reads them, in the same
frame. Between the two, the image changes job — from "being drawn into" to "being read by
shaders" — with a barrier, like the swapchain image's hand-over in § 5 (Q-25.4).

**It costs.** Six extra views of the scene, every frame: up to seven times the drawing. So the
engine only does it when asked (scop asks in chrome mode only), and the probe's faces are
small — 256 × 256 — because a reflection is a small, curved, blurry part of the screen
(Q-25.5).

**Where the analogy stops being true:** a real mirror shows the world from where *each point*
of it is. A probe shows the world from **one** point, the model's centre. On a big or long model,
reflections near its ends are slightly in the wrong place. Games accept it; so does scop.

---

## The names for it

**render to texture** — drawing into an ordinary image instead of the screen, then using the
result as a texture.

**render target** — the image a pass draws into (colour, plus its own depth image).

**reflection probe** — a point where the scene is captured into a cube map for reflections.

**cube face camera** — one of the six cameras of a probe: square view, 90° wide, looking at one
wall of the cube. Six of them see every direction exactly once.

**probe exclusion** — draws left out of the probe's views (the reflective model itself).

**offscreen pass** — a round of drawing that isn't shown (here: the six probe views), recorded
before the main pass in the same command buffer.

**layout hand-over** — the barrier that turns the probe's image from "drawn into" to "read by
shaders" (and back next frame), like § 5's swapchain transitions.

---

## How it actually works

### 1. Engine: the probe

```
ReflectionProbe (one, engine-owned): cube image 256², 6 layers, usable as
    render target AND sampled texture; a 2D view per layer (to draw into),
    a cube view (to read); its own depth image
Frame::set_reflection_probe(position)          // this frame, capture at `position`
draw calls get a flag: .in_reflections (default true); the app clears it for
    the reflective model (e.g. Frame::draw_with + exclude_from_reflections())
recording, before the main pass:
    barrier: cube → "being drawn into"
    for each face: camera = face camera at `position` → draw every included call
    barrier: cube → "read by shaders"
the environment for this frame = the probe's cube (set 0, binding 2);
    no probe this frame → the fixed environment (§ 23), unchanged
seven cameras per frame (main + 6 faces): the camera buffer holds all seven
```

### 2. scop

```
chrome mode → frame.set_reflection_probe(model centre in the world); the model's
    draws are excluded; M switches live ↔ fixed (fixed = § 23's sky)
everything else (sky, background) is in the probe's views automatically
```

### 3. Extra (owner's request, no learning section): controls on screen

```
engine: a built-in bitmap font (fixed-width, public-domain glyphs, an image made at
    start), Frame::draw_text(text, position, scale), drawn last in screen pixels by an
    overlay pipeline (no depth test, blended), reusable by any project
scop: the controls list in a corner; H shows / hides it (shown at start)
```

### Decisions

| Choice | Over | Why |
|---|---|---|
| **One probe at the model's centre** | One per object | One capture per frame; enough for "reflect what's around"; per-object probes multiply the cost |
| **Exclude the reflective draws** | Draw everything | The probe is inside the model: its own insides would fill the reflection |
| **256² faces, only when asked** | Window-sized, every frame | Reflections are small on screen; nobody pays when chrome is off |
| **Same draws, same pipelines** | Special probe shaders | Whatever the main view shows, the probe shows — that's the point |
| **M: live ↔ fixed** | Live only | Keeps § 23's look one key away, and a way to compare |
| **Overlay text from a built-in bitmap font** | A font library / TrueType | Libraries are only allowed for windows and events; a bitmap font is a small image |

---

## Known traps

- **Reflection full of dark, inside-out faces** — the reflective model wasn't excluded (Q-25.1).
- **Reflection a frame late, flickering, or a validation error about layouts** — chrome reads the cube
  before the six views are finished, or without the hand-over barrier (Q-25.4).
- **Sky upside down or sideways in the live reflection** — a face camera's "up" doesn't match the cube's
  face table (§ 23). Check: with the sky on, live and fixed chrome should look the same (M).
- **Frame rate collapses with the crowd** — the probe draws the scene six more times; the excluded crowd
  is skipped, but anything included is paid for seven times (Q-25.5).
- **Every face shows the same view** — all six face cameras got the same camera buffer: seven cameras per
  frame, one per view.
- **Text blurry or smeared** — the font image sampled with blending between texels; letters want sharp
  pixels (nearest sampling).

---

## Checkpoint

**Satisfies (subject):** bonus extra (live reflections) + controls on screen. Method: gate first, then
Claude writes it and walks you through it.

1. **Engine:** `ReflectionProbe` (cube render target + depth, per-layer and cube views);
   `Frame::set_reflection_probe`; per-draw exclusion; six offscreen passes before the main pass with
   the hand-over barriers; seven cameras per frame; the probe's cube as this frame's environment.
2. **scop:** chrome sets the probe at the model and excludes the model; M live ↔ fixed.
3. **Extra (no learning section):** engine bitmap font + `Frame::draw_text` + overlay pipeline; scop shows
   the controls, H toggles.
4. **Tests:** six face cameras look along +x, −x, +y, −y, +z, −z and together cover every direction once;
   excluded draws never reach the probe's list; no probe → the fixed environment; text layout (line
   breaks, character positions); every printable character has a glyph.

**Verify:** sky on, chrome, M — live and fixed look the same; sky off — live chrome reflects the dark
background (change `BACKGROUND` to green to see it clearly), fixed still shows the sky; FPS with chrome
live vs normal; H hides / shows the controls; validation silent.

---

## Questions

Q-25.1 (application) Why are the reflective model's own draws left out when the probe captures its cube?

Q-25.2 (application) Why is the cube captured again every frame, rather than once at start like § 23's?

Q-25.3 (prediction) Sky off, `BACKGROUND` changed to green. In chrome mode, what does the model show with
**fixed** reflections (§ 23), and what with **live** ones?

Q-25.4 (prediction) The probe's six views are drawn, but the hand-over barrier is forgotten before chrome
reads the cube. What might you see, and would validation say anything?

Q-25.5 (application) Why does the engine capture the probe only when the app asks (chrome mode), and at
256 × 256 per face rather than the window's size?

Answers: `ANSWERS.md` § 25
