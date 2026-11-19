---
section: 17
slug: bonus-complex-obj
tier: T1
status: done
prereqs: [16]
terms: [reflex corner, ear, ear clipping, polygon normal, Newell's method, flattening a polygon, point-in-triangle test, self-intersecting polygon, open edge, area check]
---

# Section 17 — Concave and non-flat faces (bonus)

**Depth: T1** — one algorithm (ear clipping), one measurement (open edges), and § 9's
cross product used on a whole face. Bonus (10 pts): *"Handle more complex `.obj` files:
concave and non-coplanar faces; render the original teapot without edge artifacts."*
**You'll be able to:** split any face into the right triangles, whatever its shape, and
prove to an evaluator whether a gap in a model is scop's fault or the file's.

> _This file was rewritten on 2026-11-19 from the code (`engine/src/triangulate.rs`,
> `ObjModel::open_edges`), `DECISIONS.md` (2026-11-18), `PROGRESS.md`, and this section's
> answers and glossary entries, after the original was lost in a commit._

---

## The idea

Since § 12, a face with more than three corners is cut into a **fan**: keep the first
corner and pair it with every next edge. That's always right for a *convex* face (every
corner points outwards, like a square or a hexagon). It goes wrong as soon as a face has
a corner that points **inwards** — an L, an arrow, a star. Some fan triangles then reach
across the notch and cover area that isn't part of the face at all.

The fix is to cut the shape the way you'd cut it out of paper with scissors: find a
corner you can snip off *cleanly* — its triangle lies entirely inside the shape — keep
that triangle, and carry on with the smaller shape that's left. A corner like that is
called an **ear**. Snip ears one by one until only three corners remain: that's the last
piece. Every piece you snipped is part of the face, and nothing outside it is.

Walk it through on the L (corners A B C D E F, going round): A isn't safe to start from
— the triangle A-C-D would cross the notch — but the corner at B is an ear: snip A-B-C.
Now the shape is A C D E F, and C, which pointed inwards before, has become an ear. Snip,
snip, and the last three corners are the last triangle. Four triangles in all — always
`corners − 2`.

Two practical problems first:

1. **"Inside" and "points outwards" are 2D ideas**, but `.obj` faces live in 3D, at any
   angle, and some are slightly **bent** (not all corners on one flat plane). So you first
   find which way the face looks — its **normal**, the arrow sticking out of it, like
   § 9's tabletop — and then look at it straight on from that side, dropping the third
   coordinate. That gives a flat 2D outline to run the tests on.
2. **Broken faces.** A file can contain a "face" whose edges cross (a bow-tie). It has no
   real inside, so there may be no ear anywhere. The loop must still end: when no ear is
   found, fan whatever is left and move on. The face is drawn, the program never hangs.

And then the teapot. The subject says "without edge artifacts", but the original teapot
has a visible slit around its lid. Is that scop? The way to answer is to **measure**: an
edge used by only one triangle is an **open edge** — a border where the surface stops
and you can see past it. A sealed model has none.

**Where the scissors analogy stops being true:** paper is always flat. A bent face has no
single "true" outline; the flattened one is a good approximation, and the triangles
still use the real 3D corners — only the *choice* of cuts is made in 2D.

---

## The names for it

**reflex corner** — a corner pointing inwards (its inside angle is over 180°). Only
concave faces have them; they're what breaks the fan.

**ear** — three consecutive corners whose triangle lies entirely inside the polygon: the
middle corner turns outwards, and no other corner is inside the triangle. Safe to cut
off. The ear is the triangle you **keep**, not a piece you throw away.

**ear clipping** — triangulating by cutting off ears one at a time until one triangle is
left. Always `n − 2` triangles for `n` corners.

**polygon normal** — the direction a face points: a *direction* (`w = 0`, § 9), not a
place.

**Newell's method** — a polygon normal computed from **every** edge of the face (three
running sums), named after Martin Newell (of the Utah teapot). A single cross product
of two edges only sees three corners: if the middle one is a reflex corner, it points
the wrong way. Newell's sums can't be fooled by one inward corner, and still give a sensible
average for a slightly bent face.

**flattening a polygon** — dropping the coordinate the normal points along most (x, y or
z), so the face becomes a 2D outline seen from its front. If the face is seen from
behind, the outline is mirrored so "turns left" still means "points outwards".

**point-in-triangle test** — a 2D point is inside a triangle if it's on the same side
(the left, for an anticlockwise triangle) of all three edges. "Which side" is a 2D cross
product — the same `turn(a, b, c)` used to check whether a corner points outwards.

**self-intersecting polygon** — a face whose edges cross, like a bow-tie. Not a valid
face; ear clipping can run out of ears, so scop fans the rest.

**open edge** — an edge used by only **one** triangle: the surface has a border there.
In a sealed model every edge is shared by exactly two triangles.

**area check** — a correct triangulation of a flat face covers exactly the face's area;
a bad fan on a concave face covers **more** (its triangles overlap and stick out).

---

## How it actually works

### 1. Where it lives

`engine/src/triangulate.rs`, called by the `.obj` parser for every face line. It's engine
code (ft_newton and ft_vox load `.obj` too) and knows nothing about scop.

```rust
pub(crate) fn triangulate(positions: &[Vec3], corners: &[u32]) -> Vec<[u32; 3]>;
```

Three corners come back unchanged. Everything else goes through the four steps below.

### 2. Newell's normal

```
n = (0, 0, 0)
for every edge a → b of the face (including last → first):
    n.x += (a.y − b.y) × (a.z + b.z)
    n.y += (a.z − b.z) × (a.x + b.x)
    n.z += (a.x − b.x) × (a.y + b.y)
```

If `n` is (almost) zero, the face has no area (all corners on a line): fan it, nothing
to clip.

### 3. Flatten

```
drop the axis where |n| is largest:
    x → keep (y, z)      y → keep (z, x)      z → keep (x, y)
those pairs keep the right-handed order, so the outline runs anticlockwise
when that component of n is positive; if it's negative, mirror the outline
```

Now "outward corner" = "the path turns left" = `turn(a, b, c) > 0`.

### 4. Clip ears

```
remaining = all corners, in order
while more than 3 remain:
    find an ear: a corner b (with neighbours a, c) where
        turn(a, b, c) > 0                              // turns outwards
        and no other remaining corner is inside a-b-c  // nothing in the way
    none found → fan the remaining corners, stop       // broken face (Q-17.4)
    keep triangle (a, b, c); remove b
keep the last 3 corners as the last triangle
```

**The search starts at corner 1.** On a convex face every corner is an ear, so this
snips (c0, c1, c2), then (c0, c2, c3), … — exactly § 12's fan. Files that were already
right (the 42 logo, both teapots) come out byte-for-byte the same; only concave faces
change.

Cost: for each ear, every other corner is tested — O(n²) per face. Real faces have a
handful of corners, so this doesn't matter.

### 5. Counting open edges

```
for every triangle, for each of its 3 edges:
    key = the two corner *positions*, smaller first        // direction doesn't matter
    count how many triangles use that key
open edges = keys used exactly once
```

Two details from the checkpoint:

- **Positions, not indices**, so two separately modelled parts that touch at exactly the
  same points count as joined.
- **−0.0 counts as 0.0.** `teapot.obj` writes 86 coordinates as `-0.000000`: the same
  point, but a different bit pattern, so a naive comparison sees two different corners.
  Adding `0.0` turns −0.0 into 0.0 before comparing. That mismatch is also a likely cause
  of the "edge artifacts" other renderers show on this file.

scop prints the count when it loads a model:
`teapot.obj: … corners, … triangles, 160 open edges, …`.

### 6. The teapot verdict

Both `teapot.obj` and `teapot2.obj` report **160 open edges**, most of them (~100)
around the lid rim: the lid and body surfaces were modelled separately and never joined.
Both files render the same in scop, and macOS Quick Look shows the same slit. So the
gap is in the model, and scop draws exactly what the file describes — from both sides,
with correct depth. Welding the near-touching corners (~0.017 apart) would close it,
but that's changing the model, not drawing it.

### Decision (2026-11-18, `DECISIONS.md`)

| Choice | Over | Why |
|---|---|---|
| **Ear clipping on the outline flattened along Newell's normal**, fan fallback | Keeping the fan | Correct for every simple face, convex or concave, flat or slightly bent; never hangs |
| | A triangulation library | Forbidden by the subject |
| **Ear search from corner 1** | From corner 0 | Convex faces keep § 12's exact triangles |
| **Draw the teapot as modelled** | Welding near-touching corners | The slit is in the file (measured), and both teapot files agree |
| **Open edges compared by position, −0.0 = 0.0** | By index / raw bits | Seams and signed zeros aren't real gaps |

Extra fix found during the checkpoint: the Makefile's `TEX` variable clashed with
make's built-in `TEX` (the TeX program), so the texture argument is now `TEXTURE`.

---

## Known traps

- **Ear clipping draws nothing, or the wrong half** — the outline runs clockwise, so
  "turns left" means "points inwards". Mirror the outline when the face is seen from
  behind (the sign of Newell's component).
- **Concave faces still wrong on tilted models** — the normal came from the first two
  edges' cross product, and the corner between them is a reflex corner: the normal is
  flipped, every test inverted. Use Newell's (Q-17.3).
- **The program hangs on one evaluator file** — a bow-tie face, no ear found, and the
  loop keeps looking. Fall back to the fan (Q-17.4).
- **Collinear corners treated as ears** — a straight "turn" (0) gives a zero-area
  triangle. Require `turn > tiny threshold`.
- **Convex models change after adding ear clipping** — the search started at corner 0,
  so a different (but still valid) set of triangles. Harmless, but start at 1 and the old
  output is kept exactly.
- **Open-edge count far too high on the teapot** — −0.0 and 0.0 compared as different
  points. Normalise the sign of zero first.
- **"The lid slit is a scop bug"** — check with the open-edge count and the second teapot
  file before changing any rendering code (Q-17.5).

---

## Checkpoint

**Satisfies (subject):** bonus — concave and non-coplanar faces; the original teapot
rendered without artifacts *that come from scop*. Method: gate first, then Claude writes
it and walks you through it.

1. **Engine:** `engine/src/triangulate.rs` — Newell's normal → flatten (mirrored when
   seen from behind) → ear clipping with the search from corner 1 → fan fallback.
   Called by the parser for every face.
2. **Engine:** `ObjModel::open_edges()`, comparing positions with −0.0 = 0.0.
3. **scop:** prints the open-edge count with the corner and triangle counts.
4. **Self-evaluation file:** `resources/test/concave.obj` — an L-shaped prism (6-corner
   caps) and a 5-point star prism (10-corner caps); 0 open edges.
5. **Tests:** L-shape and star area checks (ears cover exactly the outline; the fan covers
   more), clockwise and tilted versions, convex faces equal the old fan, a bent quad gives
   2 triangles, a bow-tie and a straight line never hang, open-edge counts (incl. signed
   zero).

**Verify (done 2026-11-18):** `make run OBJ=resources/test/concave.obj` — L and star caps
clean, no wedges across the notches; both teapots render the same, 160 open edges each;
the 42 logo unchanged; `cargo test`, `cargo clippy --all-targets` clean; validation silent.

---

## Questions

Q-17.1 (recall) What is an ear, and why does cutting off ears one at a time always end in
a correct triangulation?

Q-17.2 (application) Without looking at the screen, how can you check that a
triangulation of a flat face is correct? What does that check say about a fan from the
wrong corner of an L?

Q-17.3 (application) Why are the ear tests run on a flattened 2D outline, and why does
scop use Newell's normal instead of the cross product of the face's first two edges?

Q-17.4 (prediction) A file contains a bow-tie face (its edges cross), so no corner is an
ear. What does scop do, and why is that better than refusing the file?

Q-17.5 (application) An evaluator points at the slit around the teapot's lid and says
it's a scop bug. How do you show where the slit really comes from?

Answers: `ANSWERS.md` § 17
