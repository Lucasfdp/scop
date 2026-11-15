---
section: 13
slug: faces-and-controls
tier: T1
status: done
prereqs: [12]
terms: [flat colour per face, shared corners, duplicated corners, face of a triangle, spread-out greys, held vs pressed, per-second speed, model state, model matrix recipe, world-space movement, Euler angles, gimbal lock]
---

# Section 13 — Grey faces and moving the model

**Depth: T1** — no new GPU concepts: this section is about *data layout* (why per-face
colour needs more corners) and *app state* (keys change numbers; numbers build the § 9
matrix). Enough to finish the subject's two remaining "look and feel" requirements and
explain them at the defence.
**You'll be able to:** show every face of the logo in its own subtle grey, and move and
turn it along all three axes with the keyboard, always around its own centre.

---

## The idea

Two requirements from the subject are still missing:

1. *"Using various colors, you must make the different faces of the object visually
   distinguishable"* — and *"the faces should feature subtle shades of gray"*.
2. *"You must be able to rotate and translate it around its three main axes of symmetry
   (with the origin at the center of the object), using any user input method."*

**Greys.** Right now every *corner* has a colour, and the GPU blends the colours across
each triangle (§ 6: the rasteriser interpolates). That's why the logo looks like a smooth
rainbow. We want the opposite: one flat grey per face, with a visible edge where two
faces meet.

The catch is that a corner is **shared**: corner 2 of the logo belongs to several faces.
If corner 2 can only have one colour, every face touching it gets *that* colour at that
spot, and the greys bleed into each other. Picture a mosaic where each tile must be one
colour, but neighbouring tiles share their corner nails — and each nail is painted one
colour. You can't make the tiles distinct.

The fix: stop sharing. Give every triangle **its own three corners**, copies of the
originals, all painted the triangle's grey. Same positions, more corners. The teapot goes
from 3,644 corners to 18,960 — about 450 KB of GPU memory. Nothing.

Which grey for which face? If you just counted 0.30, 0.31, 0.32…, neighbouring faces
(which tend to be neighbours in the file too) would get nearly the same grey. Instead,
jump around the range in steps that never line up — so consecutive faces always land far
apart — and keep every grey in a middle band: subtle, never black or white.

**Controls.** The model's placement is just a few numbers: where it is (x, y, z) and how
much it's turned around each axis. Keys change those numbers a bit every frame; `draw`
builds the § 9 model matrix from them. You've already written the recipe:

```
model = move it × turn it × centre it
```

**Where the analogy stops being true:** "copies of the corners" sounds wasteful, but it's
the normal way GPUs draw flat-coloured or sharp-edged models — the same happens later
whenever a corner needs different values for different faces (normals for lighting,
texture coordinates at a seam).

---

## The names for it

**flat colour per face** — every pixel of a face gets the same colour; no blending across
it. Achieved here by giving all three corners of a triangle the same colour, so the
rasteriser's blend has nothing to blend.

**shared corners** — the § 7/§ 12 layout: each position stored once, used by every
triangle that touches it via the index list. Compact; one value per corner.

**duplicated corners** — ("un-shared", "flat-shaded" layout): each triangle gets its own
three corners. Three times the triangles' worth of vertices; each corner can carry a
value that belongs to *one* face only.

**face of a triangle** — which `.obj` face line a triangle came from. A 4-corner face
becomes 2 triangles (§ 12); both should get the *same* grey, so the parser records the
face number for every triangle.

**spread-out greys** — choosing each face's grey by stepping around a range by an
irrational fraction (here 0.618…, the *golden ratio*'s fraction), so consecutive faces get
greys far apart and no pattern repeats. The range is kept to a middle band for
"subtle shades".

**held vs pressed** — `input.held(key)`: true **every frame** the key is down (for
continuous movement). `input.pressed(key)`: true only **on the frame it goes down** (for
toggles: one press = one switch).

**per-second speed** — movement written as "units per second" and multiplied by `dt`
(§ 10): the same speed at any frame rate.

**model state** — the numbers that describe where the model is and how it's turned:
position (x, y, z) and three angles. Lives in `Scop`, changed in `update`, read in `draw`.

**model matrix recipe** — `translation(position) × rotation × translation(−centre)`.
Right to left: centre it on the origin, turn it there, then move it to `position`.

**world-space movement** — moving along the screen's axes (right is always right),
whatever way the model is turned. Comes from putting the translation card *leftmost*:
it happens after the turn. (Put it to the right of the turn and "right" means "the
model's own right" — Q-13.4.)

**Euler angles** — describing a turn as three angles, one per axis, applied in a fixed
order (here: z, then x, then y). Named after the 18th-century mathematician Leonhard
Euler. Simple and fine for a viewer.

**gimbal lock** — the known quirk of Euler angles: at some combinations (e.g. 90° around
x) two of the keys end up turning the model around the same screen axis. Harmless here;
real engines use *quaternions* (T0: a 4-number way to store a turn) to avoid it — ft_newton
territory.

---

## How it actually works

### 1. The parser records faces

`ObjModel` gains one list, filled by the same fan loop:

```rust
pub triangle_faces: Vec<u32>,   // triangle_faces[t] = which face line triangle t came from
```

A quad pushes the same face number twice. Nothing else in the parser changes.

### 2. scop builds duplicated, grey corners

```
for each triangle t with corners [a, b, c]:
    grey = face_grey(triangle_faces[t])
    push 3 vertices: positions[a], positions[b], positions[c], all with colour (grey, grey, grey)
indices = 0, 1, 2, 3, 4, 5, …       // every vertex used exactly once
```

```
face_grey(i) = 0.25 + 0.5 × fract(i × 0.618034)     // always in 0.25..0.75: subtle
```

`fract` keeps the part after the decimal point. `i × 0.618…` jumps by a bit more than
half each time and never repeats, so faces 0, 1, 2, 3… get greys like 0.25, 0.56, 0.37,
0.68… — neighbours always differ visibly.

No shader change: all three corners of a triangle have the same colour, so the
interpolation in between produces that same colour everywhere.

### 3. Controls

```
held  ← → ↑ ↓      move left/right/up/down       (x, y)
held  W S          move closer / further         (z)   (letters: + and − sit elsewhere on a Spanish keyboard)
held  I K          turn around x   (tilt)
held  J L          turn around y   (spin)
held  U O          turn around z   (roll)
pressed Space      pause / resume the automatic spin
pressed R          reset position and angles
pressed Esc        quit
```

Speeds are per second (× `dt`); movement is also × the model's `radius`, so a big teapot
and a small logo both move "about a fifth of their size per second".

### 4. The model matrix, every frame

```rust
model = translation(position)
      × rotation_y(spin + angle_y)
      × rotation_x(angle_x)
      × rotation_z(angle_z)
      × translation(−centre)
```

Right to left: centre → roll (z) → tilt (x) → spin (y, plus the automatic spin) → move.
The subject's "rotate around its main axes, origin at the centre" is the last card
working with the three turn cards; "translate" is the first card.

### Decision: per `.obj` face, not per triangle

| Option | For | Against |
|---|---|---|
| **Grey per `.obj` face** ✅ | A quad looks like one face, as the modeller drew it | One more list in the parser |
| Grey per triangle | No parser change; the subject allows it | Every quad shows a diagonal seam between two greys |
| `gl_PrimitiveID` in the fragment shader (no duplicated corners) | No extra vertices | Needs the geometry-shader feature, which MoltenVK doesn't support |

---

## Known traps

- **Faces still blend into each other** — colours set per corner on the *shared* mesh
  (Q-13.1). Duplicate the corners.
- **Model a mess of wrong triangles after duplicating** — indices still point into the
  original positions; after duplicating, the indices must be `0, 1, 2, 3, …`.
- **Every quad shows a diagonal line** — grey chosen per triangle, not per face.
- **Keys move it in jumps, or not at all** — `pressed` used for movement (one step per
  press) or `held` used for a toggle (it flips every frame while held: looks random).
- **Faster on a faster machine** — a speed not multiplied by `dt`.
- **Arrows move it in odd directions after turning it** — the translation card is to the
  right of the rotation (model-space movement, Q-13.4).
- **It swings round instead of turning in place** — the `translation(−centre)` card is
  missing or not rightmost.
- **Two keys seem to turn it the same way** — gimbal lock at that angle. Expected; press R.

---

## Checkpoint

**Satisfies (subject):** "different faces visually distinguishable", "subtle shades of
gray", "rotate and translate around its three main axes … origin at the center".
Method: gate first, then Claude writes it and walks you through it.

1. **Engine:** `ObjModel::triangle_faces`, filled by the fan loop; a test that a quad gives
   two triangles with the same face number.
2. **scop:** duplicated corners with one grey per face (`face_grey`), sequential indices;
   a test that `face_grey` stays in 0.25..0.75 and neighbours differ.
3. **scop:** model state (position, three angles, spin, spinning on/off), the key table
   above in `update`, the 5-card model matrix in `draw`. Print the controls once at start.

**Verify:**
- `cargo run -p scop -- resources/42.obj`: every face a flat grey, edges visible.
- Each key does what the table says; arrows always move along the screen's axes.
- Space pauses / resumes; R resets.
- Teapot: the same, at a comfortable speed.
- `cargo test`, `cargo clippy --all-targets` clean; validation silent.

---

## Questions

Q-13.1 (prediction) You give each face a grey, but keep the § 12 mesh where every corner
is stored once and shared. What does the logo look like, and why?

Q-13.2 (recall) After duplicating corners, how many vertices does the teapot (6,320
triangles) send to the GPU, and what do its indices look like?

Q-13.3 (application) The arrow keys use `held`, but Space uses `pressed`. Why each? What
would go wrong with Space if it used `held`?

Q-13.4 (prediction) You write the model matrix as `rotation_y(a) × translation(position) ×
translation(−centre)`. You turn the model 90° around y, then hold →. Which way does it
move on screen, and why?

Q-13.5 (application) Movement speed is `0.2 × radius × dt` per frame. Why each of the
three factors?

Answers: `ANSWERS.md` § 13
