---
section: 9
slug: maths-for-3d
tier: T1
status: done
prereqs: [2]
terms: [vector, point vs direction, dot product, cross product, normalise, matrix, transform, homogeneous coordinates, translation matrix, rotation matrix, scale matrix, composition, model matrix, view matrix, projection matrix, perspective divide, field of view, aspect ratio, near and far planes, column-major, radians, operator overloading, epsilon comparison]
---

# Section 9 — Just enough 3D maths

**Depth: T1 (your choice)** — understand what each piece does and why the order matters,
use it with ease, and test it. No derivations, no proofs: those are for ft_newton. The
subject forbids a maths library, so you'll own a small one — and every bug in it shows
up as a model in the wrong place, so tests matter more than elegance.
**You'll be able to:** describe positions and directions with vectors, move, turn and
size a model with matrices, combine them in the right order, and make far things look
smaller.

---

## The idea

Your quad's corners are numbers typed straight into NDC (§ 2's −1..+1 box). The 42 logo
won't be: its `.obj` corners are in whatever units the artist used, centred wherever
they were, and the subject wants it **turning around its own centre**, **seen in
perspective**. Every corner has to be *moved*, *turned* and *squashed onto the screen* —
every frame, for every corner. That's what matrices are for.

Start with arrows. A **vector** is three numbers (x, y, z). It can mean a **place**
("the corner is at 2 right, 1 up, 0 forward") or a **direction** — an arrow that says
*which way*, but isn't anywhere in particular ("the tabletop faces up": (0, 1, 0)).
Same three numbers, different meaning — and that difference will matter.

Picture a table. Its four corners are **places**. "Which way the top faces" is a
**direction**. Slide the table across the room: every corner's place changes, but the
top still faces up — the direction must *not* change. Turn the table on its side: now
both change — the corners move *and* the top faces sideways. So: **moving affects places
only; turning affects both.**

Now recipes. Imagine a recipe card that says "take any point and move it 3 to the
right". Another says "turn any point 30° around the vertical axis". A **matrix** is such a
card, written as a 4×4 grid of numbers; *applying* it to a point (multiplying) gives the
new point. The magic: you can **combine cards** into one card that does both — so a
whole chain of "centre it, turn it, push it away from the camera, squash it onto the
screen" becomes one matrix, applied to every corner in the vertex shader.

But combining has an order, and order changes the result. "Turn on the spot, then walk
3 steps" lands you somewhere different from "walk 3 steps, then turn". For a model:
*turn, then move away* → it spins in place over there. *Move away, then turn* → it swings
around the origin like a door. This is the most common 3D bug there is.

Last: perspective. Far things look smaller. The trick that makes the GPU do this is
almost embarrassingly simple: **divide x and y by the distance**. Twice as far, half as
big. The projection matrix's job is to set things up so the GPU can do that division
for you.

**Where the analogy stops being true:** recipe cards in a kitchen don't care about
order much. Matrices always do — `A × B` and `B × A` are generally different matrices.

---

## The names for it

**vector** — `Vec3 { x, y, z }`: three numbers. Length = how far; direction = which way.

**point vs direction** — the same three numbers mean a *place* (a corner) or a
*direction* (an arrow: "which way", not "where" — like the way a face points, used for
lighting later). Moving a model moves its places but must not change its directions;
turning it changes both. Nobody checks this by hand: the 4th number below makes the
maths do it automatically.

**dot product** — `a · b = ax·bx + ay·by + az·bz`, one number. For two length-1 vectors
it's the cosine of the angle between them: 1 = same way, 0 = at right angles, −1 =
opposite. Used later for lighting; useful in tests now.

**cross product** — `a × b`: a new vector at right angles to both. Gives a face's
"which way it points" from two of its edges — § 12/§ 17 need it.

**normalise** — scale a vector to length 1, keeping its direction: divide by its length.
The method is `normalise` too (British spelling, matching the prose).

**matrix** — a 4×4 grid of numbers: `Mat4`. Multiplying a matrix by a vector applies the
recipe; multiplying two matrices makes a combined recipe.

**transform** — any change applied to every point: move, turn, resize, project.

**homogeneous coordinates** — writing a 3D vector with a **4th number `w`**: points get
`w = 1`, directions get `w = 0`. A 3×3 grid can turn and resize but can't *move*; the
4th column holds the move, and it gets **multiplied by `w`**: for a point, ×1 = moved;
for a direction, ×0 = the move disappears. Example, moving by (5, 0, 0): new x =
x + 5·w, so the corner (1, 2, 0, **1**) → (6, 2, 0), but the arrow (0, 1, 0, **0**) →
(0, 1, 0). "Homogeneous" just names this four-number trick.

**translation matrix** — moves points by (tx, ty, tz). The numbers sit in the 4th column.

**rotation matrix** — turns points around an axis through the origin, by an angle.
One each for X, Y, Z.

**scale matrix** — stretches or shrinks along each axis: (sx, sy, sz) on the diagonal.

**composition** — combining matrices by multiplying them. With scop's convention
(below), **`A × B` applied to a point does B first, then A** — read right to left.

**model matrix** — places *this model* in the world: centre it, turn it, move it.

**view matrix** — places the world in front of the camera. scop's camera sits at the
origin looking down −z, so the view is just "push the world away from the camera".

**projection matrix** — prepares the 3D scene to be squashed onto the 2D screen, with
perspective. Together: **`P × V × M`**, applied to each corner (right to left: model,
then view, then projection).

**perspective divide** — after the vertex shader, the GPU divides x, y and z by `w`
automatically. The projection matrix sets `w` = the point's distance from the camera,
so this divide is the "twice as far, half as big" step.

**field of view** — how wide the camera sees, as an angle (vertical, in scop). Wider =
more fits on screen, everything looks smaller and more distorted at the edges.

**aspect ratio** — width ÷ height of the image. Feeding it to the projection is what
stops your quad looking stretched on a wide window (§ 6's note).

**near and far planes** — the closest and farthest distances drawn. Anything outside is
clipped. Depth (§ 11) is measured between them.

**column-major** — how a 4×4 matrix is laid out in memory: the 16 numbers stored *column
by column*. GLSL reads `mat4` this way, so scop stores it this way: `Mat4 { cols: [[f32;
4]; 4] }`, `cols[c][r]`.

**radians** — angles as distance around a circle of radius 1: a full turn is 2π ≈ 6.28,
a right angle π/2. Rust's `sin`/`cos` take radians. `degrees × π / 180`.

**operator overloading** — implementing Rust's `std::ops` traits (`Add`, `Sub`, `Mul`)
for your types, so you can write `a + b` and `m * v` instead of `a.add(b)`.

**epsilon comparison** — checking two floats are *close* (difference smaller than a tiny
`EPSILON`, e.g. 1e-5) instead of equal. `0.1 + 0.2 != 0.3` in floating point; tests
must allow for rounding.

---

## How it actually works

### 1. The convention, fixed once

- **Right-handed**: x right, y up, z towards you (the camera looks down −z). This is the
  usual 3D convention and what `.obj` files assume.
- **Column vectors**: points are columns, matrices multiply from the left: `M × p`.
  Composition reads **right to left**.
- **Column-major storage**, matching GLSL.
- **Vulkan's screen is different**: y down, depth 0..1. The projection matrix handles
  both, so everything *before* it stays in the friendly right-handed world.

Write these four lines as a comment at the top of `maths.rs`; every future bug report
starts there.

### 2. `Vec3`

`new`, `+`, `−`, `* f32`, `dot`, `cross`, `length`, `normalise`. Plain `#[derive(Clone,
Copy, Debug, PartialEq)]` — numbers, owning nothing.

### 3. `Mat4` — the cards

Written in the usual maths layout (rows on the page). In memory, scop stores the
*columns*.

```
identity            translation(t)       scale(s)
1 0 0 0             1 0 0 tx             sx 0  0  0
0 1 0 0             0 1 0 ty             0  sy 0  0
0 0 1 0             0 0 1 tz             0  0  sz 0
0 0 0 1             0 0 0 1              0  0  0  1

rotation_y(a)       (c = cos a, s = sin a; similar for x and z)
 c  0  s  0
 0  1  0  0
-s  0  c  0
 0  0  0  1
```

You don't need to know *why* those entries; you need to know **which card does what**,
and test it: rotating (1, 0, 0) a quarter turn around y gives (0, 0, −1).

Multiplying `A × B`: entry (row r, column c) of the result = dot product of A's row r
with B's column c. Sixteen dot products; a loop.

### 4. Perspective, Vulkan flavour

```
f = 1 / tan(fov_y / 2)

f/aspect   0      0                    0
0         -f      0                    0       ← minus: Vulkan's y points down
0          0      far/(near−far)       near·far/(near−far)
0          0     -1                    0       ← w = −z = distance in front of camera
```

What to remember, not derive: the last row copies the distance into `w` (so the
GPU's divide makes far things smaller); the third row squeezes distances between near
and far into Vulkan's depth range 0..1; `−f` flips y for Vulkan; `/aspect` undoes the
window's stretch. A test pins it: a point at distance `near` gets depth 0, at `far`
depth 1.

### 5. Putting a model in front of the camera

For the subject's "turn around its own centre":

```
model = translation(position) × rotation_y(angle) × translation(−centre)
           3rd: put it in place     2nd: turn       1st: centre it on the origin
view  = translation(0, 0, −distance)   // push the world in front of the camera
mvp   = projection × view × model
```

Read right to left: centre → turn → place → push away → project. § 10 sends `mvp` to the
vertex shader; § 13 adds the keys.

### 6. Tests are the point

Floats never compare equal reliably, so a helper `assert_close(a, b)` with an epsilon.
Test each card on one known point, `identity × m == m`, order matters (`T×R ≠ R×T`), the
perspective near/far depths, and `normalise` giving length 1.

---

## Known traps

- **Model swings around instead of spinning in place** — the composition order is
  reversed (`R × T` where you wanted `T × R`). Read right to left.
- **Model upside down, or moves the wrong way vertically** — the y flip done twice, or
  not at all. It's in the projection only.
- **Everything mirrored or inside-out** — a sign error in a rotation, or a left-handed
  matrix mixed with right-handed ones. Test each rotation on one point.
- **Garbage or nothing on screen after sending a matrix** — row-major data read as
  column-major (the matrix is transposed: translation lands in the bottom row, where it
  does strange things to `w`).
- **Rotation 57× too fast** — degrees given where radians are expected.
- **Tests fail on `0.30000001 != 0.3`** — use an epsilon comparison.
- **`normalise` of a zero-length vector** — divides by zero → `NaN` everywhere,
  invisible model. Guard it.
- **Stretched model** — aspect ratio missing or inverted (height ÷ width).

---

## Checkpoint

**Satisfies:** nothing visible yet — the maths "perspective" and "rotate around its centre"
rest on, unit-tested. (Method: gate first, then Claude writes it and walks you through it.)

1. `engine/src/maths.rs`: the convention comment; `Vec3` with `+ − * dot cross length
   normalise`; `Mat4` (column-major) with `identity`, `translation`, `scale`,
   `rotation_x/y/z`, `perspective`, `Mat4 × Mat4`, `Mat4 × point`.
2. Unit tests for every function, with an epsilon helper — including order-matters and
   the perspective depth checks.
3. Re-exported from the engine (`engine::Vec3`, `engine::Mat4`).
4. **Prove order matters:** a test that `translation × rotation` and `rotation ×
   translation` move the same point to different places.

**Verify:** `cargo test` — all maths tests pass; `cargo clippy` clean. Nothing on screen
changes until § 10.

---

## Questions

Q-9.1 (recall) Why does a 3D engine use 4×4 matrices and a 4th number `w`? What `w`
does a point get, what `w` does a direction get, and what difference does it make when
you apply a translation?

Q-9.2 (prediction) The model sits at (3, 0, 0). You build `model = rotation_y(angle) ×
translation(3, 0, 0)` and increase `angle` every frame. What does the model do on
screen? What did you probably want, and how do you write it?

Q-9.3 (application) The 42 logo's corners are centred on (2, 1, 0), not the origin.
Write the model matrix (as a product of cards) that spins it around its *own* centre,
in place at the origin.

Q-9.4 (prediction) Two identical cubes, one 2 units in front of the camera, one 4 units.
Roughly how big does the far one look compared with the near one, and which step makes
that happen?

Q-9.5 (prediction) Your Rust code stores matrices row by row, but GLSL reads them
column by column. Nothing else is wrong. What happens to a translation, and what might
you see?

Answers: `ANSWERS.md` § 9
