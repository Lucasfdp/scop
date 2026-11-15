---
section: 14
slug: textures
tier: T2
status: done
prereqs: [13]
terms: [texture, texel, texture coordinates, planar mapping, stretching, TGA, image header, BGR order, image origin, run-length encoding, sRGB, texture image, layout transition for upload, copy buffer to image, sampler, filtering, address mode, combined image sampler, descriptor set per texture, default white texture, mipmaps]
---

# Section 14 — Images on surfaces

**Depth: T2** — textures are the second kind of descriptor (after § 10's uniform buffer)
and the first GPU *image* you fill with your own data. You'll reuse every piece of this
in ft_vox (block textures) and beyond. You should be able to explain the path from a
file on disk to a coloured pixel, and diagnose the classic failures (upside-down,
washed-out, smeared) from a screenshot.
**You'll be able to:** load a `.tga` picture with your own loader, put it on the 42
logo, and say why each step is there.

---

## The idea

So far every pixel's colour came from its corners: the rasteriser blended the corners'
colours across the triangle (§ 6, § 13). A texture works differently: the corners say
**where on a picture** they are, and every pixel looks its colour up in the picture.

Imagine wrapping a present. The wrapping paper is the picture. For each corner of the
box, you note *which point of the paper* ends up there: "this corner is at the paper's
top-left", "this one at 30 % across, 70 % down". Those notes are just two numbers per
corner, from 0 to 1, across and down. The GPU blends *those* numbers across each
triangle — exactly like it blended colours — so every pixel knows its own spot on the
paper, and reads the colour there.

Four things have to happen:

1. **Read the picture file into memory** — your own loader, because the subject forbids
   libraries. scop uses the `.tga` format: the simplest common format that real image
   tools still save.
2. **Give every corner its paper position.** The `.obj` files have none, so scop makes
   them up: look at the model from the side where it's widest, and lay the picture flat
   over it, like a projector shining it straight through. Simple — but surfaces that run
   *along* the projector's beam get stretched into streaks. Fixing that is the § 18 bonus.
3. **Copy the picture into GPU memory** — same staging trip as the meshes (§ 7), but into a
   GPU *image* instead of a buffer.
4. **Let the fragment shader read it** — through a descriptor, like § 10's camera, plus a
   small settings object (the *sampler*) that says how to read between pixels and what to
   do past the edges.

**Where the analogy stops being true:** wrapping paper is one continuous sheet. A texture
is a grid of separate coloured squares, so "the colour at 30 % across" usually falls
*between* squares — the sampler decides whether to take the nearest one or blend the
neighbours.

---

## The names for it

**texture** — a picture used to colour surfaces. In Vulkan: a `VkImage` + view + sampler,
reached through a descriptor.

**texel** — one coloured square of a texture: **tex**ture **el**ement, like *pixel* is
**pic**ture **el**ement. Kept separate so "pixel" can keep meaning "on screen".

**texture coordinates** — a corner's position on the picture: two numbers, usually called
**U** (across, 0 = left, 1 = right) and **V** (down, 0 = top, 1 = bottom). Often just
"UVs" — the letters were free because x, y, z were taken.

**planar mapping** — making up UVs by projecting the picture straight through the model
along one axis: drop that axis's coordinate, scale the other two into 0..1.

**stretching** — on surfaces that run parallel to the projection, many texels'
worth of surface shares the same strip of UV, so the picture's edge pixels get smeared
along them. The visible flaw the § 18 bonus asks you to fix.

**TGA** — **T**ruevision **G**raphics **A**dapter (also "Targa"), a 1984 image format:
an 18-byte header, then the pixels. Easy to read by hand, still exported by GIMP,
Photoshop and macOS `sips`.

**image header** — the fixed bytes at the start of the file describing the rest: image
type, width, height, bits per pixel, and a "descriptor" byte with the origin flag.

**BGR order** — TGA stores each pixel as **B**lue, **G**reen, **R**ed (and **A**lpha if
32-bit): the reverse of RGB. The loader swaps them.

**image origin** — whether the first stored row is the **bottom** of the picture (TGA's
default) or the **top** (bit 5 of the descriptor byte set). Ignore it and the picture is
upside down.

**run-length encoding (RLE)** — simple compression: "the next 37 pixels are all this
colour" stored once instead of 37 times. TGA image type 10; GIMP often saves this way.

**sRGB** — the standard way image files store brightness: more numbers for dark shades
than light ones, because eyes are more sensitive there. The GPU must be told a texture is
sRGB (`R8G8B8A8_SRGB`) so it converts back to real brightness before the shader uses it,
and your swapchain (also sRGB, § 4) converts again on output. Get it wrong and colours
look washed out or too dark.

**texture image** — a `VkImage` with usage `TRANSFER_DST` (a copy will write into it) and
`SAMPLED` (shaders will read it), in `DEVICE_LOCAL` memory, `OPTIMAL` tiling.

**layout transition for upload** — two barriers around the copy:
`UNDEFINED → TRANSFER_DST_OPTIMAL` ("ready to be written by a copy"), then
`TRANSFER_DST_OPTIMAL → SHADER_READ_ONLY_OPTIMAL` ("ready to be read by shaders"). Same
idea as § 5's colour image, different stages.

**copy buffer to image** — `cmd_copy_buffer_to_image`: the staging buffer's bytes into the
image's texels, row by row.

**sampler** — `VkSampler`: the reading rules for a texture. Separate from the image, so one
image can be read different ways.

**filtering** — what to do when a lookup falls between texels: `NEAREST` (take the closest
one: blocky up close) or `LINEAR` (blend the four around it: smooth). scop uses `LINEAR`.

**address mode** — what to do for UVs outside 0..1: `REPEAT` (tile the picture),
`CLAMP_TO_EDGE` (stretch the edge texels). scop uses `REPEAT`.

**combined image sampler** — a descriptor type that points at an image view *and* a
sampler together. In GLSL: `sampler2D`.

**descriptor set per texture** — each texture gets its own small descriptor set (set 1 in
the shader), written once; a draw binds the set of the texture it wants. Same pointer
idea as § 10: "which picture" is chosen by which set is bound.

**default white texture** — a 1×1 white texture the engine always has, so every draw has
*some* texture bound. Multiplying by white changes nothing.

**mipmaps** — smaller pre-shrunk copies of a texture used when it's far away, to avoid
shimmering. scop skips them; mention them if asked at the defence.

---

## How it actually works

### 1. The TGA loader (engine)

```
read 18-byte header:
    byte 2       image type      2 = uncompressed true-colour, 10 = RLE true-colour
    bytes 12–13  width           (little-endian u16)
    bytes 14–15  height
    byte 16      bits per pixel  24 (BGR) or 32 (BGRA)
    byte 17      descriptor      bit 5: 1 = first row is the top
skip the image ID (length in byte 0) and any colour map (scop rejects those types)
read width × height pixels:
    type 2:  just the bytes
    type 10: packets — header byte h: if h & 0x80 → one pixel repeated (h & 0x7F) + 1 times
                                      else       → (h & 0x7F) + 1 raw pixels follow
swap BGR(A) → RGBA (alpha 255 if 24-bit)
if the origin is the bottom: flip the rows so row 0 is the top
```

Output: `Image { width, height, rgba: Vec<u8> }`. Every check that can fail on a broken
file (too short, unsupported type, zero size, truncated pixels) becomes a `TgaError` —
never a panic, same rule as the `.obj` parser.

### 2. UVs (scop)

```
find the bounding box's thinnest axis → project along it
the other two axes → (u, v), each scaled into 0..1 by the bounding box
v is flipped (1 − v) so the picture's top is at the model's top
```

For the 42 logo the thinnest axis is x, so the picture is laid over the y-z face of the
logo: front and back get the picture, the thin sides get streaks. The `Vertex` gains
`uv: [f32; 2]` (location 2, format `R32G32_SFLOAT`).

### 3. Upload (engine)

```
staging buffer (HOST_VISIBLE) ← RGBA bytes
image: R8G8B8A8_SRGB, OPTIMAL, usage TRANSFER_DST | SAMPLED, DEVICE_LOCAL memory
one-off command buffer (the § 7 Uploader):
    barrier   UNDEFINED → TRANSFER_DST_OPTIMAL
    cmd_copy_buffer_to_image
    barrier   TRANSFER_DST_OPTIMAL → SHADER_READ_ONLY_OPTIMAL   (visible to FRAGMENT_SHADER reads)
submit, wait on its fence, destroy the staging buffer
view (aspect COLOR) + descriptor set (set 1, binding 0) pointing at view + the shared sampler
```

### 4. Shaders

```glsl
// mesh.vert: pass the UV through
layout(location = 2) in vec2 in_uv;
layout(location = 1) out vec2 uv;

// mesh.frag
layout(set = 1, binding = 0) uniform sampler2D picture;
layout(location = 1) in vec2 uv;
void main() { out_colour = texture(picture, uv); }
```

`texture(picture, uv)` is the lookup: "the colour at (u, v), using the sampler's rules".
In § 15 the fragment shader will mix this with the grey; for this checkpoint it shows the
texture alone.

### 5. Engine API

```rust
let image = engine::load_tga(&bytes)?;          // your loader
let texture = gfx.create_texture(&image)?;      // → TextureId, like MeshId
frame.draw_textured(pipeline, mesh, texture, model);
frame.draw(pipeline, mesh, model);              // = default white texture
```

### Decision: TGA, set 1 per texture, planar UVs

| Choice | Over | Why |
|---|---|---|
| **TGA** (types 2 + 10) | PPM (fewer tools export it), BMP (padding rules, several header versions), PNG (needs a hand-written decompressor — a project on its own) | Simplest format real tools still save; tested at the defence with your own file |
| **Set 1 per texture** + default white | Texture in set 0 next to the camera | Camera changes per frame slot, texture per draw: different update rates → different sets (§ 10's rule again) |
| **Planar UVs from the bounding box** | Reading `vt` lines | The subject's files have none; stretch-free mapping is the § 18 bonus |
| **Embedded default texture** (`include_bytes!`) | Path at run time | Works from any directory at the defence; same reasoning as the shaders (§ 6) |

---

## Known traps

- **Picture upside down** — the origin bit ignored (or the `1 − v` flip done twice).
- **Colours swapped (blue sky looks orange)** — BGR not swapped to RGB.
- **Colours washed out / too pale** — the image created as `R8G8B8A8_UNORM` while the file's
  bytes are sRGB (or the reverse: too dark). Use `R8G8B8A8_SRGB`. (Q-14.5)
- **Diagonal garbage / skewed stripes** — the width or bytes-per-pixel wrong by one: every
  row starts a few bytes off, so the picture shears.
- **Long streaks on some faces** — planar mapping on faces parallel to the projection.
  Expected; § 18 bonus. (Q-14.2)
- **Validation error: image layout** — a barrier missing, or the descriptor written with
  a layout other than `SHADER_READ_ONLY_OPTIMAL`.
- **Validation error: descriptor set 1 not bound / not compatible** — the pipeline layout
  doesn't declare set 1, or the draw didn't bind a texture set (that's what the default
  white texture prevents).
- **Panic on a strange `.tga`** — an `unwrap` or unchecked slice. Every failure must be a
  `TgaError`.

---

## Checkpoint

**Satisfies (subject):** "apply a texture to the object" (the key and smooth fade are
§ 15). Method: gate first, then Claude writes it and walks you through it.

0. **You:** choose a cheerful picture (kitten, pony, unicorn — your choice, one you're
   allowed to use), and save it as `resources/texture.tga`. On macOS:
   `sips -s format tga picture.jpg --out resources/texture.tga`. Until then, a generated
   placeholder pattern stands in.
1. **Engine:** `engine/src/tga.rs` — `load_tga`, `Image`, `TgaError`; tests for 24/32-bit,
   both origins, RLE, and broken files.
2. **Engine:** textures — image + memory + view, upload through staging with the two
   barriers, one shared sampler, a per-texture descriptor set (set 1), the default white
   texture, `Gfx::create_texture`, `Frame::draw_textured`, everything destroyed in order.
3. **Engine:** `Vertex::uv`, attribute location 2; shaders updated.
4. **scop:** planar UVs, the embedded texture, the model drawn textured.

**Verify:**
- `cargo run -p scop -- resources/42.obj`: the picture on the front and back of the logo,
  right way up, right colours; streaks on the thin sides (expected).
- Teapot: the picture across it (stretched on the sides — expected).
- Validation silent, including at exit. `cargo test`, `cargo clippy --all-targets` clean.

---

## Questions

Q-14.1 (recall) What are texture coordinates (UVs), what range do they use, and where do
scop's come from, given the `.obj` files have none?

Q-14.2 (prediction) scop projects the picture along the logo's thinnest axis (x). What do
the logo's thin side faces look like, and why?

Q-14.3 (prediction) Your loader ignores the origin bit and the file is stored bottom row
first (TGA's default). What do you see?

Q-14.4 (application) The picture goes through a staging buffer into `DEVICE_LOCAL` memory,
with two layout transitions. Why staging (use your "how often does the CPU write it?"
rule), and what is each transition for?

Q-14.5 (prediction) You create the texture image as `R8G8B8A8_UNORM` instead of
`R8G8B8A8_SRGB`. The swapchain is still sRGB. What do the colours look like, and why?

Answers: `ANSWERS.md` § 14
