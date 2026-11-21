---
section: 22
slug: bonus-instancing
tier: T1
status: done
prereqs: [10, 15, 19, 20]
terms: [draw call, instancing, instance, instance index, first instance, instance buffer, storage buffer, draw-call overhead]
---

# Section 22 — Hundreds of copies in one draw (bonus)

**Depth: T1, minimal maths (owner's choice, 2026-11-19).** Planned as T2. This section says
what instancing is, where each piece lives and how to see it working; the matrices for each
copy are built with the cards you already have (§ 9), not new maths.
Bonus extra (owner's pick, 2026-11-18): **hundreds of spinning logos from one mesh and one draw**.
**You'll be able to:** press G and fill the screen with 512 copies of the model, each spinning
its own way — drawn with **one** draw command per object, not 512.

---

## The idea

**What a draw costs.** Every `draw_textured` so far becomes a few commands in the frame's
command buffer (§ 5): bind the pipeline, bind the mesh, push the sticky note, *draw*. Recording
those is CPU work, and the GPU has a small cost to start each draw. One draw per frame is
nothing. 512 draws of the same logo — the same mesh, the same picture, only a different place
for each — repeat the same commands 512 times for no good reason.

**Instancing** says it once: "draw this mesh **512 times**". The GPU runs the vertex shader for
every corner of every copy, and tells the shader **which copy** it's working on: the
**instance index**, 0 to 511. The shader uses it to look up that copy's own matrix in a list.
Same mesh, same picture, same pipeline — one command.

**Where the list lives.** 512 matrices don't fit on the sticky note (the push block holds about
one). They go in a **buffer** the vertex shader can read like an array: a **storage buffer**.
The CPU rewrites it every frame (the copies spin), so — exactly like the camera (§ 10) — there's
one per frame slot, or the CPU would overwrite a list the GPU is still reading.

**One shader for everything.** scop's vertex shader now always does: *sticky-note model ×
this copy's matrix*. An ordinary draw is just "1 copy, whose matrix changes nothing": the list
always starts with that do-nothing matrix (the identity, § 9) at position 0, and ordinary draws
read position 0. So there's still only one pipeline per style, and every earlier feature
(pictures, fade, debug colours, light, hot reload) works on the crowd too.

**The crowd.** G switches between the model and a crowd: an 8 × 8 × 8 grid of copies, shrunk
so the whole crowd fits where the model was, each turning at its own speed. Your controls
(arrows, W/S, I–O, the orbit) still move and turn the crowd as a whole.

**Where the analogy stops being true:** "one draw" doesn't mean "512 times cheaper". The GPU
still draws every triangle of every copy: 512 teapots are 3 million triangles. Instancing saves
the *repeating* — CPU recording and per-draw start-up — not the drawing itself.

---

## The names for it

**draw call** — one "draw this mesh" command recorded into the command buffer (`cmd_draw_indexed`).

**instancing** — drawing one mesh many times with a single draw call.

**instance** — one copy in an instanced draw.

**instance index** — which copy the vertex shader is working on (`gl_InstanceIndex`), counted from
the draw's *first instance*.

**first instance** — where in the list a draw's copies start. Ordinary draws: 0 (the identity). The
crowd: 1 onwards.

**instance buffer** — the list of per-copy data, here one matrix per copy, rewritten every frame, one
per frame slot.

**storage buffer** — a buffer a shader reads like an array of any length (`readonly buffer` in GLSL).
A uniform buffer (§ 10) is for small, fixed-size blocks.

**draw-call overhead** — the per-draw cost on the CPU (recording) and GPU (starting a draw), paid
again for every draw, however small the mesh.

---

## How it actually works

### 1. Engine: the instance list and the instanced draw

```
set 0 (per frame slot): binding 0 = camera (§ 10)   binding 1 = instance list (storage buffer)
    capacity 4096 matrices; position 0 is always the identity
Frame::draw_instances(pipeline, mesh, texture, model, params, &[Mat4])   // the copies' matrices
    ordinary draws:  first instance 0, count 1           (unchanged for every app)
    instanced draws: matrices packed after the identity, first instance = where they start
recording: write the slot's list (after its fence, like the camera) → cmd_draw_indexed(count, first)
too many copies for the list → draw what fits, warn once (never overrun the buffer)
```

### 2. scop: shader and the crowd

```
mesh.vert: layout(set = 0, binding = 1) readonly buffer Instances { mat4 instance[]; };
           model = push.model × instance[gl_InstanceIndex]
           (normals turn with the same combined model — § 20 — so the light is right per copy)
scop:      G toggles crowd;  crowd = 8 × 8 × 8 = 512 copies on a grid, shrunk to fit the model's space
           copy i: grid place, its own turn (speed and axis varied by i), its own small size
           matrices rebuilt every frame from the cards you know: move × turn × size (§ 9)
           crowd on → one draw_instances per object; off → draw_with as before
```

### Decisions

| Choice | Over | Why |
|---|---|---|
| **Storage buffer + `gl_InstanceIndex`** | A second vertex binding with `INSTANCE` rate (the textbook way) | The vertex layout (the shader contract, § 21) stays as it is; any per-copy data can be added later |
| **Identity at position 0, read by ordinary draws** | Separate instanced and plain shaders / pipelines | One pipeline per style; everything earlier works on the crowd |
| **One list per frame slot** | One shared list | The CPU rewrites it every frame — § 10's race otherwise |
| **Fixed capacity (4096), draw what fits + warn** | Grow the buffer on demand | Simple; 4096 is plenty for scop. Growing is for ft_vox |
| **Matrices built on the CPU each frame** | Spinning computed in the shader from the instance index and time | Reuses § 9's cards; the shader stays general |
| **8 × 8 × 8 = 512 copies, G toggles** | A count you can change | "Hundreds" with one key; easy to change in one constant |

---

## Known traps

- **The single model shrinks or moves when the crowd is off** — the crowd's matrices were written at
  position 0, over the identity that ordinary draws read (Q-22.3).
- **Copies flicker or jump** — one shared list for all frame slots: the CPU rewrites it while the GPU
  reads it (§ 10, Q-22.2).
- **Validation error at pipeline creation** — the vertex shader reads set 0 binding 1, but the set's
  layout doesn't have it (or not for the vertex stage). Shader and layout change together.
- **Light wrong on the copies** — normals turned by `push.model` only, not by the copy's matrix.
- **FPS still says 60** — the vsync cap (§ 19). It hides how much more the GPU does (Q-22.4).
- **Every copy identical** — the instance index not used (all read position 0), or `first instance`
  left at 0 for the crowd.

---

## Checkpoint

**Satisfies (subject):** bonus extra (instancing). Method: gate first, then Claude writes it and walks
you through it.

1. **Engine:** set 0 binding 1 storage buffer per slot (identity at 0, capacity, warn once);
   `Frame::draw_instances`; `cmd_draw_indexed` with instance count and first instance.
2. **scop:** `mesh.vert` reads the instance list; G toggles a 512-copy crowd; per-copy matrices each
   frame; controls, README.
3. **Tests:** packing (ordinary draws at 0, instanced ones after, in order); over capacity draws what
   fits; the crowd fits inside the model's sphere; copies don't overlap; 512 matrices, all different.

**Verify:** G on the logo — 512 logos, each spinning its own way; orbit, W/S, B (light per copy), C
(facing/depth per copy), N, V all work on the crowd; G off — the single model exactly as before; the
teapot crowd still runs; validation silent.

---

## Questions

Q-22.1 (recall) Drawing the logo 512 times: what does one instanced draw save compared with 512
separate draws — and what does it *not* save?

Q-22.2 (application) Why do the 512 matrices go in a buffer rather than the push block, and why one
buffer per frame slot rather than one shared buffer?

Q-22.3 (prediction) A bug writes the crowd's matrices starting at position 0 of the list. You press G
to turn the crowd off. What does the single model look like?

Q-22.4 (prediction) With G on, 512 teapots, the title still shows 60 fps. Does that mean the crowd
costs nothing? What is the number hiding?

Q-22.5 (application) Each copy turns its own way, yet the light code (§ 20) wasn't changed. Why is the
light still right on every copy?

Answers: `ANSWERS.md` § 22
