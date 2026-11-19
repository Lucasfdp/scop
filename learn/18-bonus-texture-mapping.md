---
section: 18
slug: bonus-texture-mapping
tier: T1
status: done
prereqs: [17]
terms: [box mapping, texel density, texture seam, triplanar mapping, object line, per-object bounding box, one draw per object, polygon mode, fillModeNonSolid, portability subset, point list, gl_PointSize, zoom vs dolly, scroll delta]
---

# Section 18 — Better texturing, view modes and zoom (bonus)

**Depth: T1** — mostly reuse: § 17's normals, § 14's descriptor-per-texture, § 6's pipeline
settings. Bonus (5 pts): *"Apply textures more subtly, avoiding visible stretching on certain
faces!"*, plus three extras of your choice that § 19 will list: **a picture per object**,
**solid → wireframe → points**, and **zoom**.
**You'll be able to:** texture any model without streaks, give each object in a file its own
picture, switch view modes with a key, and zoom like a camera lens.

---

## The idea

**Streaks.** Since § 14 the picture is projected straight through the whole model, like a slide
projector, along one axis. Faces facing the projector get the picture nicely; faces running
*along* its beam get one thin line of the picture smeared across them.

The fix is to use **three projectors** instead of one — front, side and top — and give every face
the one it faces most directly. A face looking sideways gets the side projector, a face looking up
gets the top one. No face is ever edge-on to its projector, so nothing streaks. You already know
how to tell which way a face looks: § 17's Newell normal.

One more source of stretching: if a picture is laid over a long thin box and squeezed to fit both
ways, it stretches. Using **the same scale** for across and down keeps the picture's squares square.

**A picture per object.** A file can contain several objects (`o name` lines — `concave.obj` has
two). Treat each as its own model for texturing: its own bounding box (so its picture fits *it*,
not the whole file) and its own **draw** — because which picture a draw reads is chosen by binding
a texture set *per draw* (§ 14). Give scop several pictures and objects take turns using them;
a key rotates the assignment.

**View modes.** The GPU can fill triangles (what we've done), draw only their edges (wireframe), or
draw only their corners (points). Each is a different pipeline (§ 6) — just a different setting —
so switching is "draw with pipeline A, B or C this frame".

**Zoom.** There are two ways to make the model bigger on screen. *Dolly*: move closer (your W/S).
*Zoom*: keep your distance, narrow the lens (the field of view, § 9). They look different: moving
close exaggerates perspective (the near side balloons); zooming in flattens it, like a telephoto
lens. scop gets a real zoom, on the mouse wheel (and keys, for laptops).

**Where the analogy stops being true:** three projectors meet at seams — where two neighbouring
faces chose different projectors, the picture doesn't continue smoothly across the edge. That's
the honest trade-off of this method (see triplanar mapping below).

---

## The names for it

**box mapping** — choosing, per face, one of three projections (along x, y or z) by the face's
normal: whichever axis the face looks along most. Also called cube or tri-axis planar mapping.

**texel density** — how many texels (§ 14) land on each unit of surface. Same density in both
directions = no stretching: scale u and v by **one** number, not each by its own box side.

**texture seam** — an edge where the picture jumps, because the faces on either side use different
projections. Box mapping always has some; they sit where the model turns a corner.

**triplanar mapping** — the smooth cousin: the fragment shader samples all three projections and
blends them by the surface direction, hiding seams. Costs 3 lookups per pixel and needs a normal
per pixel; noted for ft_vox, not done here.

**object line** — `o name` in an `.obj`: "the faces below belong to a new object". (`g name`, a
*group*, is similar; scop treats both as object boundaries.) Skipped until now (§ 12).

**per-object bounding box** — the min/max of only that object's corners: the frame its picture is
fitted to.

**one draw per object** — each object becomes its own mesh (`MeshId`) and its own `draw_textured`
call, so each can bind a different texture set.

**polygon mode** — the pipeline setting for how triangles are rasterised: `FILL` (solid),
`LINE` (edges only), `POINT` (corners only).

**fillModeNonSolid** — the device feature ("fill mode non-solid") that `LINE` and `POINT` need. It
must be *asked for* when the device is created (§ 3) — supported isn't the same as enabled.

**portability subset** — the list of things a Vulkan-on-something-else layer like MoltenVK
(Vulkan on Metal) can't do. One of them: `pointPolygons` — Metal has no "draw triangles as points"
mode, so `POLYGON_MODE_POINT` is off-limits on your Mac.

**point list** — the way round it: a pipeline whose *topology* (§ 6) is `POINT_LIST`, so every
vertex is drawn as a dot. Same mesh, same draw call, different pipeline.

**gl_PointSize** — what the vertex shader writes to say how big each dot is, in pixels. Ignored for
triangles, required for points.

**zoom vs dolly** — zoom: narrow the field of view, camera stays put (perspective flattens).
Dolly: move the camera (or the model) closer (perspective exaggerates). Film terms.

**scroll delta** — how far the mouse wheel moved this frame. Mice report "lines", trackpads report
"pixels"; the engine turns both into one number.

---

## How it actually works

### 1. Box mapping (scop)

```
per .obj face:
    normal = Newell(face)                       // § 17
    axis   = the one |normal| is largest along  // x, y or z
    keep the other two coordinates as (u, v):
        looks along x → (z, y)    along y → (x, z)    along z → (x, y)
    flip u when the face looks the "negative" way, so pictures aren't mirrored
    v = 1 − v  for the side views, so "up" stays up on the picture
scale: u, v ÷ the object's largest box side    (one number → no stretching)
       u × picture height ÷ width              (so a wide picture isn't squashed)
```

Every corner was already duplicated per triangle (§ 13), so two faces sharing a corner can give
it different UVs — exactly what box mapping needs.

### 2. Objects (engine parser + scop)

```
parser: "o name" / "g name" → start a new object; triangles remember their object
        (ObjModel::triangle_objects, object_names; no o/g lines → one object)
scop:   per object: its bounding box, its mesh (only its triangles), its picture
        pictures: scop model.obj [a.tga b.tga …] → object i uses picture (i + shift) mod count
        N: shift += 1  (rotate who gets which picture)
```

### 3. View modes (engine)

```
fill     → polygon mode FILL, topology TRIANGLE_LIST                (as now)
wireframe→ polygon mode LINE, topology TRIANGLE_LIST, needs fillModeNonSolid enabled
points   → polygon mode FILL, topology POINT_LIST, vertex shader writes gl_PointSize
```

The engine enables `fillModeNonSolid` at device creation *if* the GPU has it, and reports which
styles it can do; scop's V key cycles through those. Depth testing stays on in every mode; culling
stays off, so the wireframe shows back edges too (an X-ray look).

### 4. Zoom (engine input + scop)

```
engine: WindowEvent::MouseWheel → Input::scroll() for this frame
scop:   fov_y = clamp(fov_y × 0.9^scroll, 10°, 100°)     // and Z / X keys, per second
        R resets it to 60°
```

Multiplying (not adding) makes each wheel notch feel the same at any zoom level.

### Decisions

| Choice | Over | Why |
|---|---|---|
| **Box mapping per face** | Triplanar in the shader (smoother, 3× lookups, needs normals) | No streaks; reuses § 17; seams at corners are acceptable |
| **One scale for u and v**, picture aspect corrected | Fit each box side separately (§ 14) | That *was* a stretch source |
| **`o` and `g` as objects; pictures assigned in turn, N rotates** | Reading `.mtl` texture names (`map_Kd`) | Works with any file and any pictures you pass; `.mtl` is § 19 idea material |
| **Points via `POINT_LIST`** | `POLYGON_MODE_POINT` | Not available on Metal (portability subset `pointPolygons`) |
| **Wireframe via `POLYGON_MODE_LINE`**, feature checked | Building an edge index buffer | One pipeline setting; falls back cleanly if the GPU lacks it |
| **Zoom = field of view**, mouse wheel + keys | Moving the camera | Dolly already exists (W/S); a real zoom is the new thing |

---

## Known traps

- **Validation error creating the wireframe pipeline** — `fillModeNonSolid` not *enabled* at device
  creation (supported isn't enough) (Q-18.4).
- **Points mode shows nothing, or 1-pixel dots** — `gl_PointSize` not written, or the GPU's point
  size range is 1..1 (`largePoints` missing): check `limits.point_size_range`.
- **Picture mirrored on some faces** — the u flip for negative-facing faces missing.
- **Picture still stretched on long faces** — u and v scaled by different box sides.
- **All objects show one picture** — still one mesh / one draw; or the shift applied once instead of
  per object.
- **Seams where faces meet at an angle** — box mapping's honest cost; expected.
- **Zoom jumps, or flips the image** — field of view reaching 0° or 180°: clamp it.
- **Trackpad zoom far too fast** — pixel deltas treated as lines; scale them.

---

## Checkpoint

**Satisfies (subject):** bonus — textures without visible stretching; plus extras: a picture per
object, view modes, zoom. Method: gate first, then Claude writes it and walks you through it.

1. **Engine:** `ObjModel::{triangle_objects, object_names}` from `o`/`g` lines; pipeline styles
   (fill, wireframe, points) with the feature check; `Input::scroll()`; vertex shader writes
   `gl_PointSize`.
2. **scop:** box-mapped UVs per face with one scale and aspect correction; one mesh + draw per
   object; pictures from the command line (embedded default if none), N rotates; V cycles view
   modes; mouse wheel / Z X zoom; R resets zoom; controls list updated; README controls updated.
3. **Tests:** box mapping picks the right axis and never streaks (a face's UV area is never ~0);
   u and v share one scale; objects split correctly; zoom clamps.

**Verify:** `make run OBJ=resources/test/concave.obj` with two pictures — each shape its own picture,
N swaps them; the 42 logo's thin sides show picture, not streaks; V cycles solid → wireframe →
points; wheel zooms; validation silent.

---

## Questions

Q-18.1 (recall) Why did the § 14 projection leave streaks on the logo's thin sides, and how does
box mapping get rid of them?

Q-18.2 (application) Why must u and v be divided by the *same* number, instead of each by its own
side of the bounding box?

Q-18.3 (application) To give each object its own picture, why does each object need its own draw
call, rather than one big mesh drawn once?

Q-18.4 (prediction) You create the wireframe pipeline with `POLYGON_MODE_LINE`, but forgot to enable
`fillModeNonSolid` when creating the device. What happens, and when?

Q-18.5 (prediction) You make the model twice as big on screen two ways: once by zooming (narrowing
the field of view), once by moving closer. What looks different between the two?

Answers: `ANSWERS.md` § 18
