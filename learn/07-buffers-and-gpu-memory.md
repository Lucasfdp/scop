---
section: 7
slug: buffers-and-gpu-memory
tier: T2
status: done
prereqs: [6]
terms: [mesh, buffer, buffer usage, device memory, memory requirements, memory type, memory heap, DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT, map, bind (memory), staging buffer, cmd_copy_buffer, vertex buffer, index buffer, index type, stride, vertex binding, vertex attribute, attribute format, offset_of!, draw_indexed]
---

# Section 7 — Sending data to the GPU

**Depth: T2** — every model scop ever shows, from the 42 logo to the teapot, reaches the
GPU through exactly this path. And § 14's textures reuse the same upload code. If you
can explain where each byte goes and why, the `.obj` parser in § 12 is just "produce the
bytes".
**You'll be able to:** put vertex and index data in GPU memory, choose the right kind of
memory for it, describe its layout to the pipeline, and draw a shape made of shared
corners.

---

## The idea

Your triangle's corners live *inside the vertex shader*. That works for three points
typed by hand. It can't work for a teapot: its corners come from a file, at run time,
thousands of them. They have to travel from your program into memory the GPU can read,
and the pipeline needs to be told what they look like.

Back to the factory from § 6. Station 1 used to know its three corners by heart. Now it
gets a **box of parts** — the corner data — plus a **packing list** that says "each part
is 20 bytes: the first 8 are a position, the next 12 a colour". The box is a **vertex
buffer**; the packing list is the **vertex input description** in the pipeline.

Second idea. Most corners of a 3D model are shared: a square is two triangles, but it
only has *four* corners, not six. Instead of shipping six corners (two of them copies),
you ship the four once, plus a short list of **assembly instructions** — "triangle 1 uses
corners 0, 1, 2; triangle 2 uses 2, 3, 0". That list is an **index buffer**. For a real
model where each corner is shared by about six triangles, this saves most of the data.

Third idea: the GPU has **different kinds of memory**. Some is fast for the GPU but the
CPU can't touch it directly — the factory's **warehouse**. Some the CPU can write into
directly — the **loading dock**. (On your M2 the two are physically the same chips —
"unified memory" — but Vulkan still describes them separately, and on a campus PC with a
graphics card they really are different chips.) The usual way to deliver: the CPU puts
the goods on the loading dock, then the GPU's forklift moves them into the warehouse.
The loading-dock buffer is a **staging buffer**; the forklift is a **copy command**.

**Where the analogy stops being true:** a buffer is just a range of bytes with no
meaning of its own. "This is a position" exists only in the packing list. If the list
says the wrong thing, the GPU reads the bytes the wrong way without complaining —
you get a garbage shape, not an error.

---

## The names for it

**mesh** — a shape described as **corners plus triangles**: a list of vertices and a
list of indices saying which three corners make each triangle. The word means *net*: the
triangles form a net stretched over the shape's surface. Every 3D model in scop — the 42
logo, the teapot — is a mesh. In the engine, `MeshData` is the mesh on the CPU (two
`Vec`s), `Mesh` is the same mesh in GPU memory (two buffers).

**buffer** — `VkBuffer`: a handle describing "a run of N bytes the GPU may use for these
purposes". On its own it has **no memory**: it's a label waiting for a place.

**buffer usage** — what the buffer may be used for, declared up front:
`VERTEX_BUFFER`, `INDEX_BUFFER`, `TRANSFER_SRC` (copy *from* it), `TRANSFER_DST` (copy
*into* it), later `UNIFORM_BUFFER` (§ 10). Using it for anything not declared is invalid.

**device memory** — `VkDeviceMemory`: an actual block of memory you *allocate*, then
attach buffers to. Two objects instead of one because the GPU world separates "what
I need" from "where it lives": big engines put many buffers into one allocation.

**memory requirements** — what you ask a buffer after creating it: how many bytes it
really needs (may be more than you asked for), the alignment, and
**`memory_type_bits`** — a bit mask of which memory types it can live in (bit *i* set =
type *i* allowed).

**memory type / memory heap** — the GPU lists its kinds of memory as *heaps* (physical
pools, e.g. "the graphics card's 8 GB") and *types* (ways of using a heap, each with
property flags). You pick a type by index.

**`DEVICE_LOCAL`** — the memory is fast for the GPU (the warehouse).

**`HOST_VISIBLE`** — the CPU (*host*, in Vulkan's vocabulary) can map it and write to it
(the loading dock).

**`HOST_COHERENT`** — CPU writes become visible to the GPU automatically. Without it you
must call `flush_mapped_memory_ranges` after writing — easy to forget, so scop always
asks for coherent memory when it writes from the CPU.

**map** — `map_memory`: get a CPU pointer (`*mut c_void`) into host-visible memory.
Write through it like any C pointer, then `unmap_memory`.

**bind (memory)** — `bind_buffer_memory(buffer, memory, offset)`: attach a buffer to (a
part of) an allocation. Only once per buffer; can't be changed.

**staging buffer** — a temporary host-visible buffer used only to carry data to a
device-local buffer. Created, filled, copied from, destroyed.

**`cmd_copy_buffer`** — a recorded transfer command: copy bytes from one buffer to
another. Needs `TRANSFER_SRC` on the source and `TRANSFER_DST` on the destination.

**vertex buffer / index buffer** — buffers whose usage is `VERTEX_BUFFER` /
`INDEX_BUFFER`: the corner data, and the list of corner numbers making triangles.

**index type** — how big each index is: `UINT16` (up to 65 535 corners) or `UINT32`
(about 4 billion). scop uses `UINT32` — a detailed `.obj` can exceed 65 535 corners.

**stride** — how many bytes from the start of one vertex to the start of the next. For
scop's vertex now: 20.

**vertex binding** — `VertexInputBindingDescription`: "binding slot 0 reads a buffer
whose vertices are `stride` bytes apart, one vertex per corner" (`input rate VERTEX`).

**vertex attribute** — `VertexInputAttributeDescription`: "the value for shader
`location` N is at byte `offset` inside each vertex, in `format` F".

**attribute format** — the vertex-input formats reuse image format names:
`R32G32_SFLOAT` = **two** 32-bit **s**igned **float**s (a `vec2`), `R32G32B32_SFLOAT` =
three (a `vec3`). *R, G, B* here just mean "1st, 2nd, 3rd component", not colours.

**`offset_of!`** — a Rust macro giving the byte offset of a field inside a struct
(`offset_of!(Vertex, colour)` = 8). With `#[repr(C)]` (§ 1) the layout is C's —
fields in the order written, predictable padding — so the numbers are stable and match
what you'd compute by hand.

**`draw_indexed`** — `cmd_draw_indexed(index_count, instances, first_index,
vertex_offset, first_instance)`: draw by walking the index buffer instead of the vertices
in order.

---

## How it actually works

### The path of one vertex

```
Rust: [Vertex; 4]  ──map + copy──►  staging buffer (HOST_VISIBLE | HOST_COHERENT)
                                         │  cmd_copy_buffer (one-off command buffer,
                                         ▼   submit, wait)
                                    vertex buffer (DEVICE_LOCAL)
                                         │  bind_vertex_buffers(binding 0)
                                         ▼
pipeline vertex input: stride 20, location 0 = 2 floats @0, location 1 = 3 floats @8
                                         ▼
vertex shader: layout(location = 0) in vec2 in_position; layout(location = 1) in vec3 in_colour;
```

### 1. The vertex in Rust

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 2], // bytes 0..8
    pub colour: [f32; 3],   // bytes 8..20
}
```

`Clone, Copy` is right here (unlike `GpuContext` in § 3): a vertex is plain numbers, not
the owner of anything.

### 2. Creating a buffer with memory

1. `create_buffer`: size, usage, sharing `EXCLUSIVE`.
2. `get_buffer_memory_requirements` → size, alignment, `memory_type_bits`.
3. **Find a memory type**: for each type *i* the GPU lists, keep the first where bit *i*
   of `memory_type_bits` is set **and** its property flags contain the ones you want.
4. `allocate_memory` with the *required* size (not your data size) and that type index.
5. `bind_buffer_memory(buffer, memory, 0)`.

Destroy in reverse: `destroy_buffer`, then `free_memory`.

### 3. Uploading through a staging buffer

1. Staging buffer: usage `TRANSFER_SRC`, memory `HOST_VISIBLE | HOST_COHERENT`.
2. `map_memory` → pointer; copy the vertex bytes in (`copy_nonoverlapping`, Rust's
   `memcpy`); `unmap_memory`.
3. Final buffer: usage `VERTEX_BUFFER | TRANSFER_DST`, memory `DEVICE_LOCAL`.
4. A **one-off command buffer** from the command pool: begin (`ONE_TIME_SUBMIT`),
   `cmd_copy_buffer`, end, submit with a fence, **wait** for that fence.
5. Destroy the staging buffer and free its memory — only *after* the wait.

The index buffer takes the same path with `INDEX_BUFFER` instead of `VERTEX_BUFFER`.
This is load-time work, once per model — the wait doesn't touch the frame loop.

### Decision (made 2026-11-08): staging → device-local for meshes

| | Staging → device-local (recommended) | Host-visible only |
|---|---|---|
| Code | A copy command, a one-off submit and a wait | Map, copy, done |
| GPU read speed | Best on every GPU | Fine on your M2 (unified memory); slower on campus graphics cards, where host-visible memory sits on the CPU side |
| Reuse | **§ 14 textures need exactly this path** (images can only be filled by a copy) | Textures would need it anyway |
| When it fits | Data written once, read every frame (meshes) | Data rewritten every frame (§ 10's uniforms) |

**Chosen: staging for meshes** (logged in `DECISIONS.md`); host-visible memory for data
rewritten every frame (§ 10's uniforms). On your M2 the only cost is one extra copy at
load time — drawing speed per frame is the same.

*Deferred idea* (`00-PLAN.md` → Deferred / v2): when a GPU offers memory that is both
`DEVICE_LOCAL` and `HOST_VISIBLE` (unified memory, or Resizable BAR on PC cards), an
engine can skip staging and write directly. Worth it in ft_vox, not in scop.

### 4. Telling the pipeline

```
binding 0:   stride = size_of::<Vertex>() = 20, input rate VERTEX
location 0:  binding 0, format R32G32_SFLOAT,    offset = offset_of!(Vertex, position) = 0
location 1:  binding 0, format R32G32B32_SFLOAT, offset = offset_of!(Vertex, colour)   = 8
```

These go into the pipeline's vertex input state, which was empty in § 6.

### 5. The shader

```glsl
layout(location = 0) in vec2 in_position;
layout(location = 1) in vec3 in_colour;
layout(location = 0) out vec3 colour;

void main() {
    gl_Position = vec4(in_position, 0.0, 1.0);
    colour = in_colour;
}
```

`in` locations (from the vertex buffer) and `out` locations (to the fragment shader) are
separate numberings — both can have a 0.

### 6. Drawing

```
bind_pipeline
bind_vertex_buffers(first binding 0, [vertex buffer], [offset 0])
bind_index_buffer(index buffer, offset 0, UINT32)
draw_indexed(index_count = 6, 1 instance, first index 0, vertex offset 0, first instance 0)
```

### 7. Ownership

A `Buffer` type in the engine owns a buffer **and** its memory, and destroys both.
A `Mesh` owns a vertex `Buffer`, an index `Buffer` and the index count. Mesh buffers are
created from the device and destroyed before it, after `device_wait_idle`. (One
allocation per buffer is fine for scop; drivers allow at least 4096 allocations. A real
allocator that packs buffers into big blocks is ft_vox territory.)

---

## Known traps

- **Garbage shape, no errors** — the packing list doesn't match the bytes: wrong stride,
  wrong offset, wrong format (`R32G32B32` for a `vec2`), or a `location` that doesn't
  match the shader.
- **Struct without `#[repr(C)]`** — Rust may reorder fields. With `offset_of!` the
  numbers stay correct, but any offset you hard-coded silently goes wrong.
- **Validation error: buffer not created with `TRANSFER_DST`** (or `TRANSFER_SRC` on the
  staging buffer) — usage must name every way it's used.
- **Validation error about memory type** — you picked a type without checking
  `memory_type_bits`.
- **Allocating your data size instead of the required size** — the driver may need more
  (alignment, padding). Allocate `requirements.size`.
- **Destroying the staging buffer before the copy has run** — the submit returned at
  once; the copy happens later. Wait on the fence first. Validation reports "in use".
- **Half the triangles missing, or spikes** — index type mismatch: `u16` data declared
  as `UINT32` (or the reverse).
- **Only the first triangle right / shape scrambled** — called `draw` instead of
  `draw_indexed`: the GPU walks the vertices in order and ignores the index buffer.
- **At exit: "VkDeviceMemory has not been freed"** — destroyed the buffer but forgot
  `free_memory`.

---

## Checkpoint

**Satisfies:** "render it in a window" for data that isn't hard-coded in a shader — the
path § 12's `.obj` loader will feed. (Method: gate first, then Claude writes it and walks
you through it.)

1. Engine: a `Vertex` type (`#[repr(C)]`, position + colour) and its vertex input
   description, built with `size_of` / `offset_of!`.
2. Engine: a `Buffer` type (buffer + memory, memory-type search, destroy both).
3. Engine: upload through a staging buffer with a one-off command buffer.
4. Engine: a `Mesh` (vertex buffer, index buffer, index count) drawn with
   `draw_indexed`.
5. scop: a **quad** of 4 coloured corners and 6 indices, replacing the triangle; the
   vertex shader reads `in` attributes.
6. **Prove indices matter:** change one index, see the quad change shape, put it back.

**Verify:** a four-colour quad on the pulsing background; no validation messages,
including at exit (all memory freed); `cargo clippy` clean.

---

## Questions

Q-7.1 (recall) What's the difference between a `VkBuffer` and a `VkDeviceMemory`? Why
does Vulkan make them two separate objects?

Q-7.2 (application) A square made of two triangles: how many vertices are stored with an
index buffer, and how many indices? How many vertices without one? Why does the saving
matter much more for a teapot than for a square?

Q-7.3 (prediction) You submit the staging copy and destroy the staging buffer on the
very next line, without waiting. What might happen, and what does validation say?

Q-7.4 (application) Which memory property flags and which usage flags do (a) the staging
buffer and (b) the final vertex buffer need, and why each?

Q-7.5 (prediction) The vertex attribute for `in_colour` is declared with offset 0
instead of 8. Nothing else changes. What do you see?

Answers: `ANSWERS.md` § 7
