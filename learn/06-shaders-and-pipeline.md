---
section: 6
slug: shaders-and-pipeline
tier: T2
status: in-progress
prereqs: [5]
terms: [shader module, entry point, build script, glslc, include_bytes!, SPIR-V magic number, graphics pipeline, fixed-function state, input assembly, viewport, scissor, dynamic state, cull mode, front face, colour write mask, pipeline layout, dynamic rendering, VK_KHR_dynamic_rendering, load op / store op, gl_VertexIndex, gl_Position, location, interpolation]
---

# Section 6 — Programs that run on the GPU

**Depth: T2** — the pipeline is where most "nothing appears on screen" bugs live: one
wrong setting and you get a blank window with no error. Knowing what every setting is
for is what turns that from an evening of guessing into a minute of checking.
**You'll be able to:** write a vertex and a fragment shader, compile them into the
build, load them yourself, build a graphics pipeline, and draw your first triangle.

---

## The idea

Until now the GPU only did one thing: fill a whole canvas with a flat colour. Now it
has to *draw a shape*. You already know the stages from § 2 — vertices in, pixels out.
What's new is that **two of those stages are programs you write**, and **everything
around them is a set of switches you set once, in advance**.

Think of a production line in a factory, making one picture:

1. **Station 1 — "where do the corners go?"** You give it three corner numbers (0, 1,
   2). For each, a small program *you wrote* says where on screen that corner lands,
   and maybe attaches a colour to it. This program is the **vertex shader**. It runs
   once per corner: three times for a triangle.
2. **Station 2 — "which pixels are inside?"** A machine (not your program) joins the
   corners into a triangle and lists every pixel it covers. For a pixel in between the
   corners, it *blends* whatever the corners carried: halfway between a red corner and
   a blue corner gets purple. This is **rasterisation**, and the blending is
   **interpolation**.
3. **Station 3 — "what colour is this pixel?"** For each covered pixel, another small
   program *you wrote* decides the final colour. This is the **fragment shader**. It
   runs once per covered pixel — thousands of times for a big triangle.
4. **Station 4 — "write it into the picture"**, with switches like "which colour
   channels may be written" and "should it blend with what's already there".

Here's the Vulkan twist: you don't configure the line station by station while it runs.
You describe **the whole line, every switch, in one go**, and Vulkan builds it into one
frozen object — the **graphics pipeline**. Why frozen: the driver can look at the whole
line at once and compile it into the most efficient form for this GPU. The price: change
any switch, and you need a different pipeline. A few switches can be left "to be set
while recording" — those are **dynamic state**.

Two more things, before the details. Your shader programs are written in GLSL (§ 2), but
Vulkan only accepts the compiled SPIR-V form — so a compiler, `glslc`, turns them into
bytes when you build, and **your own code** loads those bytes into Vulkan. And to draw,
you now tell the GPU "start drawing into *this* image" — with **dynamic rendering**,
which you chose over render passes (`DECISIONS.md`).

**Where the analogy stops being true:** the stations don't process one corner, then one
pixel, in order. The GPU runs thousands of copies of your shaders at the same time, with
no guaranteed order between them. A shader can't see what its neighbours are doing.

---

## The names for it

**shader module** — `VkShaderModule`: Vulkan's handle for one block of SPIR-V code. You
create it from SPIR-V words, use it while building the pipeline, and may destroy it
straight after — the pipeline keeps its own compiled copy.

**entry point** — the function in a shader where it starts, like `main` in C. In GLSL
it's always `void main()`; you name it (`c"main"`) when building the pipeline.

**build script** — a file named `build.rs` next to a crate's `Cargo.toml`. Cargo
compiles and runs it **before** compiling the crate. Ours runs `glslc` on every shader.
`cargo:rerun-if-changed=shaders` (printed by the script) tells Cargo to run it again
whenever that folder changes.

**`glslc`** — **GLSL c**ompiler, from Google's `shaderc` project (installed by Homebrew
as `shaderc`). `glslc triangle.vert -o triangle.vert.spv`. A tool you run, not a library
scop links to.

**`include_bytes!`** — a Rust macro (the `!` marks a macro: code that writes code at
compile time) that copies a file's bytes into the program as a `&'static [u8]`. Paired
with `env!("OUT_DIR")` — the folder where Cargo tells build scripts to put generated
files — it embeds the compiled shaders in the binary. No shader files to find at
runtime.

**SPIR-V magic number** — every SPIR-V file starts with the 32-bit word `0x07230203`.
Checking it is the first line of defence against loading the wrong file. SPIR-V is
defined as a list of 32-bit *words*, so a valid file's size is a multiple of 4.

**graphics pipeline** — `VkPipeline` created with `create_graphics_pipelines`: the
frozen production line — shaders plus every fixed-function switch.

**fixed-function state** — the switches for the stations you don't program: how corners
become shapes, how shapes become pixels, how pixels are written. "Fixed function"
because these used to be hardware that did one fixed job.

**input assembly** — how the stream of corners is grouped into shapes. *Topology*
`TRIANGLE_LIST`: every 3 corners make one separate triangle.

**viewport** — which rectangle of the image NDC's −1..+1 box (§ 2) is stretched onto.
Normally the whole image: x 0, y 0, width × height of the swapchain extent.

**scissor** — a rectangle outside of which pixels are thrown away. Normally the whole
image too. (Viewport *squashes* the picture into a rectangle; scissor *cuts* it.)

**dynamic state** — switches deliberately left out of the frozen pipeline, set while
recording instead. scop makes **viewport and scissor dynamic**, so the same pipeline
still works when the window size changes (§ 16).

**cull mode** — whether to throw away triangles that face away from you (*back-face
culling*). Saves work on closed 3D models. For a single flat triangle: `NONE`.

**front face** — which corner order counts as "facing you": `COUNTER_CLOCKWISE` or
`CLOCKWISE`, as seen on screen. Only matters when culling is on — and it's the classic
reason a model is invisible: culling on, winding the other way.

**colour write mask** — which colour channels (R, G, B, A) the pipeline may write.
Defaults to *none* in Vulkan's create-info struct: forget it, and everything is drawn
but nothing is written. `RGBA` = write all four.

**pipeline layout** — `VkPipelineLayout`: the list of *extra inputs* shaders can read
besides vertices (uniforms, textures — § 10, § 14). Empty for now, but a pipeline must
have one.

**dynamic rendering** — starting a drawing pass with `cmd_begin_rendering`, naming the
image views to draw into right there in the command buffer. No render-pass or
framebuffer objects.

**`VK_KHR_dynamic_rendering`** — the device extension that provides it on Vulkan 1.2
(MoltenVK). In Vulkan 1.3 it's built in. You enable the extension **and** switch on its
*feature* (`dynamic_rendering = true`) when creating the device — extensions add
functions; features are switches that must also be turned on.

**load op / store op** — what happens to the image's contents at the start and end of
drawing. `LOAD_OP_CLEAR` = fill with a clear colour first (replaces § 5's
`cmd_clear_color_image`); `STORE_OP_STORE` = keep the result (you want to show it).
The alternatives `LOAD`/`DONT_CARE` matter later for performance.

**`gl_VertexIndex`** — a GLSL built-in input: *which corner* this run of the vertex
shader is for (0, 1, 2…). It lets you draw a triangle with no vertex data at all — the
positions live in the shader. (`gl_` = built in, from OpenGL.)

**`gl_Position`** — the built-in output the vertex shader must write: where the corner
lands, as a `vec4` (x, y, z, w). For now `w = 1.0` and `z = 0.0`; § 9–10 explain what
they're for.

**location** — `layout(location = 0)`: a numbered slot connecting a vertex shader output
to a fragment shader input. Same number on both sides = same value. Names don't matter;
numbers do.

**interpolation** — blending per-corner values across the triangle for each pixel,
weighted by how close the pixel is to each corner. Done by the rasteriser, for every
`out` the vertex shader writes.

---

## How it actually works

### The pieces and where they live

```
scop/shaders/triangle.vert ─┐   build.rs runs glslc     ┌─ OUT_DIR/triangle.vert.spv
scop/shaders/triangle.frag ─┴──────────────────────────►└─ OUT_DIR/triangle.frag.spv
                                                                 │ include_bytes!
                                                                 ▼
scop (main.rs)  ── bytes ──►  engine: bytes → SPIR-V words → shader modules
                                         → pipeline layout + graphics pipeline
                                         → draw_frame: begin_rendering, bind, draw(3)
```

Shaders belong to the app (scop decides what to draw); building pipelines from them
belongs to the engine (every project needs it).

### 1. The shaders

```glsl
// triangle.vert
#version 450                                   // GLSL version the compiler should use
layout(location = 0) out vec3 colour;          // slot 0 → the fragment shader

vec2 positions[3] = vec2[](vec2(0.0, -0.5), vec2(0.5, 0.5), vec2(-0.5, 0.5));
vec3 colours[3]   = vec3[](vec3(1, 0, 0),    vec3(0, 1, 0),  vec3(0, 0, 1));

void main() {
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
    colour = colours[gl_VertexIndex];
}
```

```glsl
// triangle.frag
#version 450
layout(location = 0) in vec3 colour;           // slot 0 ← interpolated from the corners
layout(location = 0) out vec4 out_colour;      // slot 0 → colour attachment 0

void main() {
    out_colour = vec4(colour, 1.0);
}
```

`vec2/vec3/vec4` = vectors of 2/3/4 floats. Remember **Vulkan's y points down**: y = −0.5
is the *top* corner.

### 2. Compiling them: `scop/build.rs`

For each `.vert`/`.frag` file in `scop/shaders/`: run `glslc <file> -o
$OUT_DIR/<file>.spv`; if `glslc` is missing or fails, stop the build with its error
message. Print `cargo:rerun-if-changed=shaders`.

### 3. Loading them: our own code

`include_bytes!` gives bytes; Vulkan wants 32-bit words. The loader:
1. checks the length is a non-zero multiple of 4;
2. turns each group of 4 bytes into a `u32`, **little-endian** (lowest byte first — how
   SPIR-V is stored and how `glslc` writes it on every machine you'll use);
3. checks the first word is the magic number `0x07230203`.
Then `create_shader_module` with those words. (A plain `&[u8]` → `&[u32]` cast would be
undefined behaviour: the bytes aren't guaranteed to start at a 4-byte-aligned address.)

### 4. Turning dynamic rendering on

- **Choosing the GPU:** also require `VK_KHR_dynamic_rendering`, and check the feature is
  supported (`get_physical_device_features2` with a
  `PhysicalDeviceDynamicRenderingFeatures` chained on — the `push_next` idea from § 3).
- **Creating the device:** enable the extension, and chain
  `PhysicalDeviceDynamicRenderingFeatures { dynamic_rendering: true }` onto the device
  create info.
- **Calling it:** on a Vulkan 1.2 device the functions come from the extension's own
  function table, `ash::khr::dynamic_rendering::Device` (as with the swapchain in § 4) —
  not from `Device` directly.

### 5. Building the pipeline

One `GraphicsPipelineCreateInfo`, filled from these parts:

| Part | Setting |
|---|---|
| Shader stages | vertex module + fragment module, entry `main` |
| Vertex input | empty (positions are in the shader) |
| Input assembly | `TRIANGLE_LIST` |
| Viewport / scissor | count 1 each, values **dynamic** |
| Rasterisation | `FILL`, cull `NONE`, front face `COUNTER_CLOCKWISE`, line width 1.0 |
| Multisample | 1 sample (no anti-aliasing) |
| Colour blend | blending off, write mask `RGBA` |
| Dynamic states | `VIEWPORT`, `SCISSOR` |
| Layout | the empty pipeline layout |
| Rendering info (chained) | one colour attachment, format = **the swapchain's format** |

The last row is dynamic rendering's replacement for a render pass: the pipeline only
needs to know the *formats* it will draw into.

### 6. Recording a frame now

```
barrier: UNDEFINED → COLOR_ATTACHMENT_OPTIMAL
         (dst stage COLOR_ATTACHMENT_OUTPUT, dst access COLOR_ATTACHMENT_WRITE)
begin_rendering: colour attachment = this image's VIEW, layout COLOR_ATTACHMENT_OPTIMAL,
                 load CLEAR (clear colour), store STORE, area = whole extent
bind_pipeline(GRAPHICS, pipeline)
set_viewport, set_scissor        (whole extent)
draw(3 vertices, 1 instance, first vertex 0, first instance 0)
end_rendering
barrier: COLOR_ATTACHMENT_OPTIMAL → PRESENT_SRC_KHR
         (src stage COLOR_ATTACHMENT_OUTPUT, src access COLOR_ATTACHMENT_WRITE)
```

And the submit now waits on `image_free` at `COLOR_ATTACHMENT_OUTPUT` — the first stage
that writes to the image. The swapchain no longer needs `TRANSFER_DST`.

*Instance* in `draw(3, 1, …)`: drawing the same shape several times in one call. One copy
here.

### 7. Ownership

The pipeline and its layout are created from the device, so they're destroyed before it
(and after `device_wait_idle`). Shader modules can be destroyed right after the pipeline
is created.

---

## Known traps

- **Blank window, no errors** — the most common causes, in order: colour write mask left
  empty; culling on with the corners in the wrong order; the triangle is off-screen
  (positions outside −1..+1); forgot `cmd_draw`.
- **Triangle upside down** compared with a tutorial — the tutorial is OpenGL (y up).
  Vulkan's y points down.
- **"Dynamic state not set"** validation error (e.g. *vkCmdDraw: … VK_DYNAMIC_STATE_VIEWPORT
  … not set*) — you made viewport/scissor dynamic but didn't call `cmd_set_viewport` /
  `cmd_set_scissor` in this command buffer.
- **Format mismatch** validation error at `begin_rendering` or draw — the pipeline's
  colour format isn't the swapchain's format.
- **`cmd_begin_rendering` crashes or "function not loaded"** — you called it through
  `Device` on a 1.2 device. Use the extension's function table.
- **Validation error that `dynamicRendering` feature isn't enabled** — you enabled the
  extension but not the feature.
- **Build fails: "glslc: not found"** — `glslc` isn't installed or isn't on `PATH`
  (`brew install shaderc`).
- **Shader edits don't show up** — the build script didn't rerun. Check it prints
  `cargo:rerun-if-changed=shaders`.
- **`invalid magic number` from your loader** — you embedded the GLSL text, not the
  `.spv` output.
- **Fragment shader input never set / garbage colours** — the `location` numbers don't
  match between the two shaders.

---

## Checkpoint

**Satisfies:** "render it in a window" — the first real shape on screen.
(Method: gate first, then Claude writes it and walks you through it.)

1. `scop/shaders/triangle.vert` and `triangle.frag`; `scop/build.rs` compiling them with
   `glslc`; shaders embedded with `include_bytes!`.
2. Engine: our own SPIR-V loader (length, little-endian words, magic number) + unit
   tests for it.
3. Engine: dynamic rendering checked at GPU choice, enabled (extension + feature) at
   device creation.
4. Engine: a pipeline built from shader bytes the app passes in; destroyed in `Drop`.
5. `draw_frame` switches from `cmd_clear_color_image` to dynamic rendering: clear via
   load op, draw 3 vertices. Swapchain drops `TRANSFER_DST`.
6. **Prove the write mask matters:** set it to empty, run, see a background with no
   triangle, put it back.

**Verify:** an RGB triangle (red top, green bottom-right, blue bottom-left) on the
pulsing background; no validation messages; `cargo test` passes the loader tests;
`cargo clippy` clean.

---

## Questions

Q-6.1 (recall) For one triangle that covers 1000 pixels: how many times does the vertex
shader run, how many times does the fragment shader run, and what does each one decide?

Q-6.2 (prediction) The three corners are red, green and blue. What colour is a pixel
right in the middle of the triangle, and which part of the line produced that colour —
your code or the GPU's?

Q-6.3 (prediction) You copy the corner positions from an OpenGL tutorial: top corner at
y = +0.5. Where does that corner appear in your window?

Q-6.4 (application) Name two settings in the pipeline (or in recording) that must match
the swapchain, where each value comes from, and what you'd see if one were wrong.

Q-6.5 (prediction) You change `triangle.frag` to output pure red and run `cargo run`
without touching any Rust file. Does the triangle turn red? Explain what happens between
saving the file and the window opening.

Answers: `ANSWERS.md` § 6
