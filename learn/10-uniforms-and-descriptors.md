---
section: 10
slug: uniforms-and-descriptors
tier: T2
status: in-progress
prereqs: [8, 9]
terms: [uniform, per-frame vs per-draw data, push constant, push constant range, stage flags, pipeline layout, uniform buffer, persistently mapped, descriptor, descriptor set layout, descriptor pool, descriptor set, set and binding numbers, descriptor write, binding a descriptor set, std140, view-projection matrix]
---

# Section 10 — Passing per-frame data to shaders

**Depth: T2** — this is how *every* piece of outside data reaches a shader from now on
(camera, model, textures in § 14, the fade value in § 15, lights in ft_vox). You should
be able to say where each value lives, why, and what breaks if it's in the wrong place.
**You'll be able to:** send § 9's matrices to the vertex shader, and see a cube spinning
in perspective.

---

## The idea

So far the vertex shader has only had **per-corner** inputs: each corner's own position
and colour, read from the vertex buffer (§ 7). But the matrix from § 9 is different: it's
the **same for every corner** of the cube. Every corner gets turned by the same angle and
pushed away by the same distance. The shader needs a way to get "settings for the whole
draw", on top of "data for this corner".

You *could* apply the matrix on the CPU and re-upload all the corners every frame. For an
8-corner cube that's fine. For the teapot it means rewriting 3,644 corners 60 times a
second, and the GPU's whole job is to do that maths in parallel. So the plan is: corners
stay in GPU memory untouched (uploaded once, § 7), and every frame you send just the
matrices, a few dozen bytes.

There are two ways to hand those settings to the GPU. Think of a kitchen:

- **A sticky note stuck to the order ticket.** It's tiny, and it travels *with* the order,
  so every order can carry a different note for free. In Vulkan the "order ticket" is the
  command buffer (§ 5). The note is copied into it while you record.
- **A noticeboard on the kitchen wall.** It can hold much more, and many orders can read
  the same board. But the cook has to be told *which* board to look at, and if you rewrite
  the board while a cook is still reading last night's menu from it, they get a mix of
  both.

scop uses both. The **camera** (how the world is seen) changes once per frame and is the
same for every object, so it goes on the noticeboard. The **model matrix** (how *this*
object is placed and turned) is different for every object, so it goes on a sticky note.

**Where the analogy stops being true:** a real noticeboard is read "now". The GPU reads
it *later*, when it actually runs the frame, which may be after the CPU has already
started preparing the next one (§ 5: two frames in flight). That's why scop gets **one
noticeboard per frame slot**, not one shared board. More in "How it actually works" § 3.

---

## The names for it

**uniform** — a shader input that has the *same value for every corner* (and every pixel)
of a draw. "Uniform" as in "the same throughout", the opposite of the per-corner inputs
from the vertex buffer. In GLSL it's the keyword `uniform`.

**per-frame vs per-draw data** — *per-frame* changes at most once per frame and is shared
by every draw (camera). *Per-draw* can differ between two draws in the same frame (each
object's model matrix). Sorting values into these two buckets decides where they go.

**push constant** — the sticky note. A small block of bytes written straight into the
command buffer with `cmd_push_constants`. *Push* because you push the bytes into the
recording; *constant* because the value doesn't change during the draw that reads it.
Vulkan guarantees at least **128 bytes** on every GPU (the limit is called
`maxPushConstantsSize`, "maximum push constants size"). That's two `mat4`s exactly.

**push constant range** — `VkPushConstantRange`: the declaration "my shaders will read
push-constant bytes from offset 0 to 64, in the vertex stage". It lives in the pipeline
layout (below).

**stage flags** — `VkShaderStageFlags`: which shader stages can see an input.
`VERTEX` = the vertex shader, `FRAGMENT` = the fragment shader. Declaring the wrong
stage is a validation error.

**pipeline layout** — `VkPipelineLayout`. You've had an *empty* one since § 6. It's the
pipeline's list of outside inputs: "this many noticeboards shaped like this, plus this
much sticky-note space". Shaders may only read inputs that the layout declares. Think of
it as the function signature for the pipeline's extra parameters.

**uniform buffer** — a normal `VkBuffer` (§ 7) created with the usage flag
`UNIFORM_BUFFER` ("a shader will read this as uniform data"). It's the noticeboard.
OpenGL calls it a **UBO**, "Uniform Buffer Object", and you'll see that name everywhere.

**persistently mapped** — *mapping* (§ 7) gives the CPU a pointer into GPU-visible memory.
*Persistently* means map it **once** at creation and keep the pointer forever, instead of
map → write → unmap every frame. Fine for `HOST_COHERENT` memory, where writes become
visible to the GPU without an extra flush call.

**descriptor** — a tiny record that *describes* a resource to a shader: "the buffer is
*this* `VkBuffer`, starting at byte 0, 64 bytes long". It's the note telling the cook
which board to read. It doesn't contain the data; it **points at** it.

**descriptor set** — `VkDescriptorSet`: a group of descriptors that get attached
together, in one call. scop's has one descriptor in it (the camera buffer). § 14 adds the
texture to a set.

**descriptor set layout** — `VkDescriptorSetLayout`: the *shape* of a set without the
actual resources: "binding 0 is one uniform buffer, visible to the vertex stage". It's
like a struct declaration; the descriptor set is an instance of it.

**descriptor pool** — `VkDescriptorPool`: where descriptor sets are allocated from. Same
idea as the command pool (§ 5): you size it up front ("room for 2 sets holding 2 uniform
buffer descriptors in total"), and destroying the pool frees every set in it.

**set and binding numbers** — the shader's address for a descriptor:
`layout(set = 0, binding = 0)` means "descriptor set slot 0, entry 0 inside it". Same idea
as `location` for vertex inputs (§ 7), with two levels.

**descriptor write** — `VkWriteDescriptorSet` + `update_descriptor_sets`: filling in a
set, i.e. making "binding 0 of set A" point at a particular buffer. Done **once**. After
that you change the *buffer's contents*, never the descriptor.

**binding a descriptor set** — `cmd_bind_descriptor_sets`: a command recorded into the
command buffer saying "from here on, set slot 0 is *this* set". Like `cmd_bind_pipeline`
(§ 6), it stays in effect for the following draws.

**std140** — the rule GLSL uses to lay out a uniform block's bytes in memory: **st**andar**d**
layout, introduced in GLSL version **1.40**. For a `mat4` it means 16 floats, 64 bytes,
**column by column**. That's exactly how § 9's `Mat4` is stored (`#[repr(C)]`, `cols[c][r]`),
so the bytes can be copied across as they are. Other types have padding traps (a `vec3`
takes 16 bytes, not 12); scop avoids them by only sending `mat4`s.

**view-projection matrix** — `projection × view` multiplied on the CPU *once per frame* into
one card: "see the world from the camera, then squash it onto the screen". The shader
then only multiplies by two matrices, not three.

---

## How it actually works

### 1. The shader side

The vertex shader gains two inputs and goes 3D:

```glsl
#version 450

// Noticeboard: set 0, binding 0. Written once per frame by the engine.
layout(set = 0, binding = 0) uniform Camera {
    mat4 view_proj;   // projection × view
} camera;

// Sticky note: pushed before each draw.
layout(push_constant) uniform Push {
    mat4 model;       // this object's place and turn
} push;

layout(location = 0) in vec3 in_position;   // was vec2: corners are 3D now
layout(location = 1) in vec3 in_colour;
layout(location = 0) out vec3 colour;

void main() {
    gl_Position = camera.view_proj * push.model * vec4(in_position, 1.0);
    colour = in_colour;
}
```

Read the `gl_Position` line right to left, like § 9: the corner (a **point**, so `w = 1.0`)
is placed by the model matrix, then seen through the camera. `Camera` and `Push` are just
names for the blocks; `camera` and `push` are what the code uses.

### 2. The pipeline layout gets filled in

Until now: `create_pipeline_layout(&PipelineLayoutCreateInfo::default())`, an empty
signature. Now it declares both inputs:

```
descriptor set layout:  binding 0 → 1 × UNIFORM_BUFFER, stage VERTEX
push constant range:    stage VERTEX, offset 0, size 64   (one mat4)
pipeline layout:        set 0 = that set layout  +  that push range
```

The engine builds **one** pipeline layout and every pipeline uses it. Bonus: when two
pipelines share a layout, a descriptor set bound once stays valid as you switch between
them, so it's bound once per frame, not once per draw.

If a shader reads something the layout doesn't declare (or declares the wrong stage or
size), pipeline creation reports a validation error. The layout and the GLSL must agree.

### 3. One camera buffer per frame slot

Remember § 5: there are `FRAMES_IN_FLIGHT = 2` frame slots. While the GPU is still drawing
frame N from slot A, the CPU is already recording frame N+1 into slot B. If both slots
shared one camera buffer, the CPU would be rewriting the matrix **while the GPU is still
reading it** for the previous frame.

So each slot owns its own uniform buffer:

```
slot A: command buffer, fence, image_free … + camera buffer A + descriptor set A → buffer A
slot B: command buffer, fence, image_free … + camera buffer B + descriptor set B → buffer B
```

When is it safe to write buffer A? After waiting on slot A's fence: that's the moment the
GPU has finished everything previously submitted from slot A, and `draw_frame` already
waits there (step 1). So the write goes **after the fence wait**, before recording.

The memory is `HOST_VISIBLE | HOST_COHERENT` and persistently mapped: per-frame data is
written by the CPU every frame, so the staging route from § 7 would cost more than it
saves. That's the "revisit" note in the § 7 decision.

### 4. Descriptor pool and sets: set up once

At startup, after the buffers exist:

1. Create the descriptor set layout (the shape from step 2).
2. Create a descriptor pool with room for **2 sets** and **2 uniform-buffer descriptors**.
3. Allocate 2 sets from it, both with that layout.
4. **Write** each set: set A's binding 0 → buffer A (offset 0, 64 bytes); set B's → buffer B.

That's the last time the descriptors change. Every frame you only change the 64 bytes
*inside* the buffer; the descriptor already points there.

### 5. Every frame

```
wait slot fence  →  acquire image  →  reset fence           (unchanged, § 5)
copy view_proj (64 bytes) into this slot's mapped buffer     NEW
record:
  begin rendering, viewport, scissor                         (unchanged)
  for each draw:
    bind pipeline                                            (unchanged)
    bind descriptor set: slot 0 = this slot's set            NEW (cheap; fine per draw)
    push constants: model, 64 bytes, VERTEX stage            NEW
    bind vertex/index buffers, draw_indexed                  (unchanged)
submit, present                                              (unchanged)
```

Binding the set inside the loop is the simple, always-correct version. Binding it once
before the loop works too because every pipeline shares the layout. Either is fine for scop.

### 6. Corners go 3D

`Vertex::position` becomes `[f32; 3]`, its attribute format becomes `R32G32B32_SFLOAT`
("**R**ed **G**reen **B**lue channels, **32** bits each, **S**igned **FLOAT**": the colour
names are just Vulkan's way of saying "3 floats"), and the shader reads `vec3`. The old
quad gets `z = 0.0`.

### 7. The engine API

What scop sees (handles only, no Vulkan, as in § 8):

```rust
frame.aspect()                         // width ÷ height of this frame's image
frame.set_camera(projection * view)    // once per frame → the slot's uniform buffer
frame.draw(pipeline, mesh, model)      // per draw → push constant
```

`aspect()` is read **every frame** from the swapchain size, so a resized window gets the
right projection without any extra code. `Mat4` gets one small method to hand its 16
floats over as bytes in column order. Its layout already matches std140.

### 8. What scop does with it

```
update:  angle += speed × dt                    (seconds, so speed is the same on any machine)
draw:    projection = perspective(60°→radians, frame.aspect(), 0.1, 100.0)
         view       = translation(0, 0, −3)       // camera 3 units back from the cube
         frame.set_camera(projection × view)
         model      = rotation_y(angle) × rotation_x(angle × 0.5)
         frame.draw(pipeline, cube, model)
```

The cube's corners are centred on the origin already, so no centring card is needed. The
42 logo will need one (§ 13, Q-9.3).

### Decision: push constants for the model, uniform buffer for the camera

| Option | For | Against |
|---|---|---|
| **Both matrices as push constants** (2 × 64 = 128 bytes) | Fewest objects: no buffers, no descriptors | Uses the *entire* guaranteed space: § 15's fade value wouldn't fit next to them. Descriptors still have to be learned in § 14 for the texture, on a harder resource |
| **Both in uniform buffers** | One mechanism | Per-draw data in buffers needs one buffer region per object (or "dynamic offsets"), more bookkeeping for nothing |
| **Camera in a uniform buffer, model as a push constant** ✅ | Each value sits where it belongs (per-frame vs per-draw); learns descriptors on the simplest resource before § 14; leaves 64 bytes of push space; the pattern real engines and ft_vox use | More Vulkan objects (set layout, pool, sets, 2 buffers) |

Recommended: the third. Logged in `DECISIONS.md`.

---

## Known traps

- **Validation error at pipeline creation mentioning the push constant range or a
  descriptor that isn't in the layout** — the GLSL and the pipeline layout disagree:
  missing push range, size not 64, or the wrong stage flag (`FRAGMENT` instead of `VERTEX`).
- **Validation error at draw time: descriptor set not bound** — `cmd_bind_descriptor_sets`
  missing (or recorded after the draw).
- **Camera never moves although you write a new matrix every frame** — writing into the
  wrong slot's buffer, a buffer that isn't `HOST_COHERENT`, or a pointer from a map
  that was later unmapped.
- **Occasional one-frame jumps or jitter when the camera moves fast** — one buffer shared
  between frame slots, or writing before the fence wait. The validation layer normally
  says **nothing** about this: it's a race, not an API misuse.
- **Cube distorted, flipped, or gone with no error** — the matrix arrives transposed
  (Q-9.5). std140 `mat4` is column-major and so is `Mat4`; don't add `row_major` in GLSL
  and don't transpose on upload.
- **Cube stretched on a wide window** — `aspect` computed once in `setup`, or
  height ÷ width instead of width ÷ height.
- **Cube rotates 57× too fast or wildly** — degrees given to `perspective` or a rotation.
- **Back faces drawn over front faces at some angles** — *expected* at this checkpoint.
  The GPU draws triangles in index order and nothing says which is in front yet. That's
  § 11's whole job. Different colours per corner make it easy to see.
- **Validation error at shutdown: descriptor pool / set layout / buffer not destroyed** —
  the new objects need destroying after `device_wait_idle`, before the device. Destroying
  the pool frees its sets; you don't free sets one by one.

---

## Checkpoint

**Satisfies (subject):** "perspective" and "rotates around its main symmetrical axis"
(on a cube for now; the 42 logo arrives in § 12–13). Method: gate first, then Claude
writes it and walks you through it.

1. **3D corners:** `Vertex::position: [f32; 3]`, attribute format `R32G32B32_SFLOAT`,
   shader `in vec3`. Quad updated to `z = 0`.
2. **Engine:** one descriptor set layout + one push constant range in a shared pipeline
   layout; one persistently mapped `HOST_VISIBLE | HOST_COHERENT` uniform buffer per frame
   slot; a descriptor pool and one set per slot, written once; `draw_frame` writes the
   camera after the fence wait; `record_frame` binds the set and pushes each draw's model
   matrix; everything new destroyed in the right order.
3. **API:** `Frame::aspect()`, `Frame::set_camera(Mat4)`, `Frame::draw(pipeline, mesh, model)`;
   `Mat4` → column-order floats for upload. No Vulkan types leak to scop.
4. **scop:** a cube (8 corners, 36 indices, a different colour per corner), spinning
   with `dt`, in perspective, camera 3 units back.

**Verify:**
- `cargo run -p scop`: the cube spins smoothly; far edges look smaller than near ones.
- Resize the window: still not stretched.
- Validation layer silent, including at exit (Esc).
- Some faces overlapping wrongly at certain angles: expected, see § 11.
- `cargo test` and `cargo clippy --all-targets` clean.

---

## Questions

Q-10.1 (recall) scop sends two matrices to the vertex shader in two different ways. Which
matrix goes where, and why does each one belong there?

Q-10.2 (prediction) To save memory, you use **one** camera uniform buffer for both frame
slots instead of one each. Nothing else changes. What can go wrong, why, and would the
validation layer tell you?

Q-10.3 (application) In `gl_Position = camera.view_proj * push.model * vec4(in_position, 1.0);`
what does the `1.0` mean, and in what order are the transforms applied to the corner?

Q-10.4 (prediction) The descriptor set for each slot is written once at startup and never
touched again, yet the shader sees a new camera matrix every frame. How is that possible?
What *would* need a new descriptor write?

Q-10.5 (prediction) `aspect` is computed once in `setup` and stored. The cube spins
happily, then you drag the window to twice its width. What do you see, and where should
`aspect` come from instead?

Answers: `ANSWERS.md` § 10
