---
section: 12
slug: obj-parser
tier: T1
status: done
prereqs: [11]
terms: [Wavefront OBJ, vertex line, face line, 1-based index, negative index, face token, texture coordinate and normal lines, material library, triangulation, fan triangulation, convex and concave, bounding box, model centre, framing, parse error with line number]
---

# Section 12 — Reading .obj files

**Depth: T1** — the format is simple and you only need part of it. What matters is
reading it *correctly* (off-by-one is the classic bug) and *robustly*: the subject warns
that evaluators will load other `.obj` files during the defence.
**You'll be able to:** load the 42 logo and the teapot from disk, turn them into
`MeshData`, and see them spin, centred and fitting the window.

---

## The idea

An `.obj` file is just text, one instruction per line. Here's the start of
`resources/42.obj`:

```
# Blender v2.71 (sub 0) OBJ File: '42.blend'
mtllib 42.mtl
o Cube
v 0.232406 -1.216630 1.133818
v 0.232406 -0.745504 2.843098
...
f 1 2 3
f 16 2 3 17
```

Two kinds of line do all the work:

- **`v x y z`** — "here is a corner". The corners are numbered in the order they
  appear: the first `v` line is corner **1**, the second is corner **2**, and so on.
- **`f a b c …`** — "here is a face, made from these corners". `f 1 2 3` is a triangle
  from corners 1, 2 and 3. `f 16 2 3 17` is a face with **four** corners.

That's exactly what your `MeshData` already wants (§ 7): a list of corners, and a list
of indices where every 3 make a triangle. The parser's whole job is translating one into
the other. Two small differences get in the way:

1. The file counts from **1**; Rust arrays count from **0**. Every index needs `− 1`.
2. Faces can have 4 (or more) corners; the GPU only draws triangles. A 4-corner face
   gets cut into 2 triangles.

Everything else in the file (comments, object names, materials, smoothing groups) scop
can **skip**. The subject only asks for shape, perspective, greys and a texture — and
the greys come from the subject, not from the material file.

Once the model is loaded, two practical problems remain. The logo's corners run from
z = 0 to z = 2.84, so its middle isn't at the origin — spinning it would swing it around
like Q-9.2's door. And the teapot is about 6 units wide, so a camera 3 units back would
be *inside* it. Both are fixed with one measurement: the smallest box that contains every
corner. Its middle is the model's centre; its size tells the camera how far back to sit.

**Where the analogy stops being true:** "every 3 indices make a triangle" is *your*
format; `.obj` has no such rule — a face line can have any number of corners, and a few
`.obj` features (negative indices, `1/2/3` tokens) look nothing like plain numbers. The
parser has to handle the format as written, not as you'd wish it were.

---

## The names for it

**Wavefront OBJ** — the `.obj` text format, from Wavefront Technologies' 1980s
animation software. Still everywhere because it's trivial to read and write.

**vertex line** — `v x y z`: one corner's position. The `v` is for **v**ertex.

**face line** — `f a b c …`: one polygon, listing its corners by number. The `f` is for
**f**ace.

**1-based index** — counting that starts at 1, not 0. `.obj` corner numbers are 1-based,
so corner `n` is `positions[n − 1]`.

**negative index** — `f -3 -2 -1` means "the last three corners defined *so far*": −1 is
the most recent `v` line. Rare, but legal — an evaluator's file might use it.
Corner `−k` = `positions[len − k]` at the moment the face is read.

**face token** — each item after `f`. It can be `5`, `5/2`, `5/2/7` or `5//7`: corner /
texture coordinate / normal. scop needs only the part before the first `/`.

**texture coordinate and normal lines** — `vt u v` (where on an image this corner sits;
§ 14 makes its own, § 18 may read these) and `vn x y z` (which way the surface faces —
a *direction*, § 9's `w = 0`). Skipped for now.

**material library** — `mtllib 42.mtl` and `usemtl Material`: colours and shininess
in a separate `.mtl` file. Skipped: the subject wants greys, not the file's colours.

**triangulation** — cutting a polygon into triangles.

**fan triangulation** — the simplest way: keep the first corner fixed and pair it with
each next edge. Corners `a b c d e` → `a b c`, `a c d`, `a d e`. Like folding a paper fan
open from one point.

**convex and concave** — *convex*: every corner points outwards, like a square or a
hexagon; a fan always works. *Concave*: some corner points inwards, like an L or an arrow;
a fan can produce triangles that stick *outside* the shape. Handling those properly is
the § 17 bonus.

**bounding box** — the smallest box, aligned with the x/y/z axes, that contains every
corner: the minimum and maximum of each coordinate.

**model centre** — the middle of the bounding box: `(min + max) / 2`. It's what the
subject means by "the center of the object" for rotation.

**framing** — choosing the camera distance so the whole model fits the window.

**parse error with line number** — `42.obj:17: face uses corner 50, but only 42 are
defined`. A broken evaluator file must give a clear message and a clean exit, never a
panic or a crash.

---

## How it actually works

### 1. Where it lives

The parser goes in the **engine** (`engine/src/obj.rs`), because ft_newton, ft_vox and
friends will load `.obj` files too. It knows nothing about colours or scop:

```rust
pub struct ObjModel {
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,   // 0-based, ready for the GPU
    pub min: Vec3,                  // bounding box
    pub max: Vec3,
}
impl ObjModel {
    pub fn centre(&self) -> Vec3;   // (min + max) / 2
    pub fn radius(&self) -> f32;    // half the box's diagonal: "how big"
}
pub fn parse_obj(text: &str) -> Result<ObjModel, ObjError>;
```

scop then turns an `ObjModel` into `MeshData` — choosing the colours is scop's business
(and § 13's topic).

### 2. Line by line

```
for (line_number, line) in text.lines().enumerate():
    drop everything from '#' on; split on whitespace
    match the first word:
        "v"  → read 3 floats (ignore a 4th, if any) → positions.push
        "f"  → read every token:
                 take the part before '/'
                 parse as an integer n
                 n > 0 → index = n − 1          (1-based)
                 n < 0 → index = len + n        (relative: −1 = last so far)
                 n = 0 or out of range → error with line number
               fewer than 3 corners → error
               fan: for i in 1..count−1 → triangle (c0, c[i], c[i+1])
        anything else → skip   (o, g, s, mtllib, usemtl, vt, vn, l, …)
    no faces at all at the end → error ("nothing to draw")
```

`text.lines()` also strips Windows `\r\n` line endings, so files saved on Windows work.

### 3. Centre and framing

After parsing: bounding box → centre → radius. In scop:

```
model  = rotation_y(angle) × translation(−centre)        // Q-9.3: centre it, then turn
distance = radius / sin(fov_y / 2)                        // far enough for the whole
                                                          // sphere around it to fit
view   = translation(0, 0, −distance)
far    = distance + radius                                // so the back isn't clipped
```

You don't need to derive the `sin` line: "bigger model or narrower view → sit further
back" is the whole idea.

### 4. Errors reach the user cleanly

`ObjError` carries the line number and a reason, and implements `Display`. `main`
prints `scop: resources/broken.obj:17: …` and returns a failure exit code — the same
path as a missing file today. No `unwrap` on anything that came from the file.

### Decision: what the parser supports

| Supported now | Skipped (and why) |
|---|---|
| `v`, `f` with any number of corners | `vt` / `vn`: not needed until § 14/18 |
| `5/2/7`, `5//7` tokens (first part used) | `mtllib` / `usemtl`: the subject wants greys |
| negative indices | `l` (lines), `p` (points): nothing to draw as triangles |
| comments, blank lines, `\r\n` | concave polygons drawn correctly: § 17 bonus |

---

## Known traps

- **Model is a tangle of spiky triangles** — the `− 1` is missing (or applied twice):
  every face connects the wrong corners. (Q-12.3)
- **`index out of bounds` panic** — same bug, last corner; or an evaluator's file with a
  bad index. The parser must check and return an error instead.
- **Holes in the model, or only half of each quad drawn** — 4-corner faces taken as one
  triangle (only the first 3 corners). Fan-triangulate.
- **Odd triangles sticking out of a face** — fan triangulation on a concave face. Fine for
  the mandatory files; it's the § 17 bonus.
- **Model swings around instead of spinning** — not centred (Q-9.2/9.3).
- **Nothing visible, or only a slice of the model** — camera inside the model or the far
  plane too close: use the framing distance and `far = distance + radius`.
- **Model lying on its side** — the file's "up" isn't +y (Blender's own up is +z). Not a
  parser bug; a fixed rotation card in scop if it matters (§ 13).
- **Panic on a weird file instead of a message** — an `unwrap()` on a parse result.
  Every failure must become an `ObjError` with a line number.

---

## Checkpoint

**Satisfies (subject):** "parse the .obj and render it correctly"; "the origin at the
centre of the object". Method: gate first, then Claude writes it and walks you through it.

1. **Engine:** `engine/src/obj.rs` — `parse_obj`, `ObjModel` (positions, 0-based
   triangles, bounding box, `centre`, `radius`), `ObjError` with line numbers. Unit tests:
   triangle, quad → 2 triangles, `1/2/3` tokens, negative indices, comments/unknown lines
   skipped, index 0 / out of range / too few corners / no faces → errors.
2. **scop:** read the file from the command line, parse it, print `n corners, m triangles`,
   build `MeshData` (colour per corner from its position, like the cube — real greys are
   § 13), centre it, frame it, spin it. A parse error → message + failure exit code.
3. The cube code goes.

**Verify:**
- `cargo run -p scop -- resources/42.obj`: the logo, spinning around its own centre,
  fully in view.
- `… resources/teapot.obj` and `teapot2.obj`: the teapot, the same.
- A broken file (`echo "f 1 2 3" > /tmp/bad.obj`): a clear message, no panic.
- `cargo test`, `cargo clippy --all-targets` clean; validation silent.

---

## Questions

Q-12.1 (recall) Which two kinds of line does scop's parser actually use, and what does
it do with every other line? Give two examples of lines it skips.

Q-12.2 (application) The line is `f 16 2 3 17`. Which triangles (as 0-based indices) does
the parser produce?

Q-12.3 (prediction) You forget the `− 1`. What would the 42 logo look like, and what
happens when the parser reaches a face that uses corner 42?

Q-12.4 (prediction) A face is shaped like an L (6 corners, one pointing inwards) and gets
fan-triangulated from its first corner. What can go wrong on screen?

Q-12.5 (application) The logo's corners run from z = 0 to z = 2.84. Write the model matrix
(as cards) that spins it around its own centre at the origin, and say where the centre
comes from.

Answers: `ANSWERS.md` § 12
