# Answers — scop

Read after attempting. Answers explain the reasoning, not just the fact.

## Section 1

**A-1.1** `&T` is a shared (read-only) borrow; `&mut T` is a mutable (read-write)
borrow. At any moment a value can have **either** any number of `&T` **or** exactly
one `&mut T` — never both. The reason: if something can write while others read,
the readers can see half-updated data, or data that has been moved or freed.

**A-1.2** It does **not** compile (`E0502`). `first` is a shared borrow into the
vector, and `push` needs a mutable borrow while `first` is still used on the last
line. The rule prevents a **use-after-free**: `push` can reallocate the vector's
buffer to a new address, leaving `first` pointing at freed memory — exactly what
happens in C with a pointer into a `realloc`ed array. Fix: finish using `first`
before pushing, or copy/clone the value out first.

**A-1.3** `instance` is destroyed first, because struct fields are dropped in
**declaration order**. Vulkan requires the reverse: every object created from the
instance (the device included) must be destroyed *before* the instance. Destroying
the instance first leaves the device referring to a dead parent — the validation
layer reports it, and on some drivers it crashes. Fix: declare fields in reverse
creation order, or (clearer, and what § 3 does) implement `Drop` on the parent
struct and destroy everything explicitly in the correct order.

**A-1.4** `?` returns the error *as is* from your function, so the error types must
match — or the error must be convertible with `From`. Your function's error type is
`String`, but the error is `std::io::Error`, and there is no automatic conversion.
Fixes (any one):
- convert by hand: `fs::read_to_string(path).map_err(|e| e.to_string())?`
- or make your own error enum with `impl From<std::io::Error> for MyError`, so `?`
  converts automatically — the better choice for the engine crate, because callers
  can then match on *which* error happened.

**A-1.5** `CString::new("scop").unwrap()` creates a **temporary** `CString` that is
dropped at the end of the statement. `.as_ptr()` borrows into it as a *raw* pointer,
which the borrow checker doesn't track — so the code compiles, but `p` is dangling
from the next line onwards. Passing it to Vulkan reads freed memory: garbage
application names, or a crash. Fix: `let name = CString::new("scop")?;` on its own
line, then `name.as_ptr()`, and keep `name` alive until Vulkan is done with it.

## Section 2

**A-2.1** The **vertex shader** runs once per vertex; the **fragment shader** runs
once per fragment (each pixel a triangle covers, before the depth test). Everything
else — primitive assembly, rasterisation, depth test — is fixed function: you
configure it, you don't program it.

**A-2.2** A cube has 6 faces; each square face is split into 2 triangles, so 12
triangles × 3 vertices = **36**. Corners are shared by several faces but get sent
once per triangle that uses them. (In § 7 an **index buffer** lets you send the 8
unique corners once and 36 small indices instead — but for per-face colours you'll
often need separate vertices per face anyway, since a corner shared by three faces
would need three different colours.)

**A-2.3** A smooth **gradient**: pure red at one corner, green and blue at the
others, blending across the inside. **Rasterisation** does it: it interpolates every
vertex output across the triangle, weighted by the pixel's distance to each corner,
before the fragment shader sees it. This is why per-face flat colours need extra
work (§ 13) — by default, everything gets blended.

**A-2.4** You might see the **old data, the new data, or a mix** — flickering,
stretched triangles, or a garbled frame — and it can change from run to run. Submit
returns immediately; the GPU reads the vertex data *later*, while your Rust code
has already moved on. You've changed memory the GPU is still reading. This is a
**race between CPU and GPU**; the fix is synchronisation (fences, § 5) and keeping
one copy of per-frame data per frame in flight.

**A-2.5** The person appears **upside down**. Vulkan's NDC has +y pointing *down*
the screen; OpenGL's has +y pointing *up*. A shader written for OpenGL puts "up" at
the bottom. (Depth is different too: Vulkan expects 0..1, OpenGL −1..1, which can
clip or distort depth.) Both are fixed once in your projection matrix (§ 10), not by
flipping the model.

## Section 3

**A-3.1** Creation: **instance → (debug messenger) → logical device** — the physical
device is *chosen*, not created, and the queue is *fetched* from the device.
Destruction is the mirror: **device → debug messenger → instance**. Children before
parents, always: everything made from the device must go before the device, and the
device and messenger are made from the instance.

**A-3.2** MoltenVK is a *portability* driver, and the loader hides portability drivers
unless you opt in. Windows tutorials never need to. Fix: (1) enable the instance
extension `VK_KHR_portability_enumeration`, and (2) set the
`ENUMERATE_PORTABILITY_KHR` flag in the instance create info. Then, on the device,
enable `VK_KHR_portability_subset` because the device lists it.

**A-3.3** **Family 1** — the only one with `GRAPHICS`, and drawing needs a graphics
queue. Families 0 and 2 can't draw. (Real engines sometimes also use a dedicated
transfer-only family like 2 for fast uploads, but that's an optimisation scop doesn't
need.)

**A-3.4** **Nothing.** In a release build `cfg!(debug_assertions)` is false, so your code
never enables the validation layer — there is no inspector, so nobody notices the leak.
The program exits normally and the buffer is cleaned up by the OS with the process.
This is why you test with debug builds, and why "it runs without errors" in release
proves nothing about correctness.

**A-3.5** The first `destroy_device` destroys it; the second destroys an already
destroyed object — **undefined behaviour**: a crash, a validation error (if the layer
is on), or silent corruption. It's a double free, Vulkan edition: a handle is just a
number, so copying it doesn't copy the device. Rust helps if the engine never hands out
raw `VkDevice` handles to own: one struct owns the device and destroys it in its
`Drop`, and that struct is **not `Copy` or `Clone`** — so there can only ever be one
owner, and one destroy.

## Section 4

**A-4.1** The surface is the adaptor between Vulkan and a platform's window: "a place I
can present images to". Core Vulkan doesn't know about windows because it must run
everywhere — servers with no screen, phones, consoles, every desktop window system —
and each has a different idea of a window. So windows are an extension
(`VK_KHR_surface` plus one per platform, e.g. `VK_EXT_metal_surface`).

**A-4.2** **3** — `min_image_count + 1`. `max_image_count = 0` is a special value
meaning *no upper limit*, not "zero images allowed", so there is nothing to clamp to.
The `+1` gives a spare image so the CPU/GPU can start the next frame without waiting
for the presentation engine to release one.

**A-4.3** Your logical window is 1280×720 but the Retina screen has twice as many
physical pixels: 2560×1440. Asking for 1280×720 either triggers a validation error (the
surface reports a fixed current extent you must match) or gives an image a quarter of
the size, stretched — blurry, or drawn into one corner. Use the **physical** size:
`current_extent` from the surface capabilities, or `window.inner_size()` when the
current extent is the special "you choose" value.

**A-4.4** Minimising makes the window 0×0. A swapchain with a zero extent is invalid:
creation fails or the validation layer reports an error, and some drivers crash. Fix:
when the size is zero, skip rendering and rebuilding until the window has a real size
again.

**A-4.5** Because the instance and the device choice now depend on the window:
(1) **creating the instance** needs the window-system extensions, which you get from the
window's display handle, and (2) **choosing the physical device / queue family** needs
the surface, which is created from the window — you must check the family can present
to it. Since `winit` only gives you the window in `resumed`, Vulkan setup moves there.

## Section 5

**A-5.1** **Fence:** the GPU signals it when a submitted batch finishes; the **CPU**
waits on it (`wait_for_fences`). It's how your code knows it may reuse a command buffer.
**Semaphore:** signalled by the GPU (or by `acquire`, when the image comes off screen);
waited on by the **GPU** (a submit) or the **presentation engine** (a present). The CPU
never waits on a binary semaphore — it only names which ones to use.

**A-5.2** It **hangs on the first frame**. The first thing the frame does is
`wait_for_fences` on its slot's fence — but nothing has ever been submitted with that
fence, so nothing will ever signal it. The CPU waits forever: the window opens but stays
blank and stops responding. Creating fences `SIGNALED` means "the previous (imaginary)
use of this slot is already finished", so the first wait passes at once.

**A-5.3** **wait for fence → acquire → reset fence → record → submit → present.**
- *acquire* is told to raise `image_free` when the image is really off screen.
- *submit* **waits on** `image_free` (at the transfer stage), **raises** `render_done`
  for that image, and switches on the fence when the commands finish.
- *present* **waits on** `render_done`.
Reset comes after acquire so an early return on out-of-date never leaves the fence off
with no job to switch it on.

**A-5.4** Validation reports that the presented image is in the wrong layout — roughly
"`vkQueuePresentKHR`: image is in `VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL`, expected
`VK_IMAGE_LAYOUT_PRESENT_SRC_KHR`" (`VUID-VkPresentInfoKHR-pImageIndices-01430`). Without
validation the result is **undefined**: often it looks fine (many desktop drivers use the
same memory arrangement for both), sometimes you get garbage, black, or the previous
frame. "It looked fine on my machine" is exactly how this bug reaches the defence on a
different GPU.

**A-5.5** With one frame in flight the CPU records frame N, submits it, and at the start
of frame N+1 **waits on the fence until the GPU has finished frame N** — it sits idle
while the GPU draws, and then the GPU sits idle while the CPU records. A second set of
objects (command buffer, fence, `image_free`) lets the CPU record frame N+1 into the
*other* command buffer while the GPU is still drawing frame N, so both stay busy. It
only waits when it gets two whole frames ahead.

## Section 6

**A-6.1** The vertex shader runs **3 times** — once per corner — and decides *where each
corner lands* (`gl_Position`) plus what values it carries (the colour). The fragment
shader runs **about 1000 times** — once per covered pixel — and decides *that pixel's
colour*. That ratio is why expensive per-pixel work belongs in the vertex shader when it
can: three runs instead of a thousand.

**A-6.2** A **roughly equal mix of red, green and blue — a greyish white** (about a third
of each). Neither of your programs mixed it: the **rasteriser** (fixed-function, the
GPU's part) interpolated the three corner colours, weighted by distance, and handed the
mixed value to your fragment shader, which just wrote it out. (With an sRGB swapchain,
the mix looks lighter than a third would suggest — § 4 / § 5.)

**A-6.3** **At the bottom.** OpenGL's y axis points up, Vulkan's NDC y points down, so
+0.5 is halfway towards the *bottom* edge. The whole triangle appears upside down
compared with the tutorial's screenshot. Fix: flip the sign of y (or, later, put the
flip in the projection matrix, § 10).

**A-6.4** Any two of:
- **Colour attachment format** in the pipeline's rendering info ← the swapchain's
  `format`. Wrong → validation error at `begin_rendering`/draw; without validation,
  undefined (garbage or nothing).
- **Viewport and scissor** (set while recording) ← the swapchain's `extent`. Wrong size
  → the triangle squashed into a corner (viewport) or cut off (scissor) — the Retina
  "quarter of the window" look from § 4.
- **Render area** in `begin_rendering` ← the extent. Too small → only part of the image
  is cleared and drawn.

**A-6.5** **Yes.** Saving changes a file in `scop/shaders/`; `cargo run` sees that the
build script asked to rerun when that folder changes (`cargo:rerun-if-changed=shaders`),
so it runs `build.rs`, which runs `glslc` and writes a new `.spv` into `OUT_DIR`. The
`include_bytes!` in `main.rs` depends on that file, so scop is recompiled with the new
bytes embedded, and the window shows a red triangle. If `glslc` reports an error in your
GLSL, the *build* fails with that message — you never get a stale binary.

## Section 7

**A-7.1** A **`VkBuffer`** is a description — "N bytes, usable as a vertex buffer and as
a copy destination" — with no storage. A **`VkDeviceMemory`** is an actual allocated
block of memory, of one memory type. You bind the buffer to (part of) the memory. They're
separate so the "what" and the "where" can be chosen independently: you ask the buffer
what kind of memory it needs, pick a type, and — in bigger engines — put many buffers
into one large allocation, because the number of allocations is limited and each one is
slow.

**A-7.2** With an index buffer: **4 vertices, 6 indices** (two triangles × 3). Without:
**6 vertices**, two of them duplicates. For a square, that saves little. For a teapot,
each corner is shared by around six triangles, so without indices you'd store each
corner ~6 times: indices cut the vertex data by roughly that factor, and the GPU can
reuse already-processed vertices instead of running the vertex shader again for the
copies.

**A-7.3** The submit returns immediately; the copy runs on the GPU **later**. Destroying
the staging buffer (and freeing its memory) right away means the GPU may copy from
memory that's already freed — **undefined behaviour**: garbage vertices, a corrupted
shape, or a crash. Validation reports that the buffer (or memory) is destroyed while
**still in use** by a pending command buffer. Fix: wait on the upload's fence (or the
queue) before destroying it.

**A-7.4** (a) **Staging:** memory `HOST_VISIBLE` (the CPU must map and write it) and
`HOST_COHERENT` (so the writes are visible to the GPU without a manual flush); usage
`TRANSFER_SRC` (it's copied *from*). (b) **Final vertex buffer:** memory `DEVICE_LOCAL`
(fastest for the GPU, read every frame); usage `VERTEX_BUFFER` (bound as vertex input)
and `TRANSFER_DST` (it's copied *into*).

**A-7.5** The **shape stays correct** (positions are still read from offset 0), but the
**colours are wrong**: each corner's "colour" is read from bytes 0..12 — its position x,
its position y, and its real red value. So the colour becomes (x, y, red). Negative
coordinates give negative values, which end up as 0 in the image, so corners on the left
or top lose red/green. No error anywhere — the GPU reads bytes exactly as the packing
list says.

## Section 8

**A-8.1** The **engine owns the main loop** (via `winit`'s event loop) and **calls scop**:
`setup` once when the GPU is ready, `update` with the input and delta time each frame,
`draw` to collect the frame's draw list. scop never runs a loop or calls "render now";
it only answers when called. That reversal — framework calls app, not app calls library —
is inversion of control.

**A-8.2** **Engine:** (b) the swapchain, (d) the SPIR-V loader, (f) translating `winit`
key codes — every project needs them unchanged. **scop:** (a) the `.obj` path, (c) the
backdrop colour, (e) the GLSL of scop's shaders, (g) which key toggles the texture —
choices this project makes. (The `.obj` *parser* is engine — ft_newton loads models too —
but *which file* is scop's choice.)

**A-8.3** scop dropping the `Mesh` would run its `Drop`/destroy and free the buffers
while up to two frames on the GPU may still be reading them — **use-after-free on the
GPU**: validation reports buffers destroyed while in use; without it, garbage or a crash.
With a handle, the **engine keeps ownership**: scop holds only a number, and the engine
destroys meshes in its own `Drop`, after `device_wait_idle`. scop can't destroy what it
doesn't own.

**A-8.4** With `winit`'s `KeyCode` in the trait, **every app** that names a renamed key
— scop, ft_newton, ft_vox — fails to compile and must be edited, even though none of them
cares about `winit`. With the engine's own `Key`, only **one place** changes: the
engine's translation from `winit` codes to `Key`. The apps don't notice. That's what
loose coupling buys.

**A-8.5** **`pub`:** `Vertex` (apps build mesh data from it), `MeshId` (apps receive and
pass it back), `App` (apps implement it). **`pub(crate)` or private:** `Buffer` (GPU
memory is engine business; apps get `MeshId`s instead), `find_memory_type` (an internal
helper of `Buffer`), `GpuContext::draw_frame` (the engine calls it from its own loop; an
app calling it directly would bypass the frame order).

## Section 9

**A-9.1** A 3×3 matrix can rotate and scale but **can't move** (translate) anything —
it has nowhere to put "add 3 to x". Adding a 4th row and column gives that room, and a
4th number `w` on every vector. **Points get `w = 1`, directions get `w = 0`.** The
translation part of the matrix is multiplied by `w`, so points move and directions
don't — "up" stays "up" wherever the model is placed. (The same `w` later carries the
distance for the perspective divide.)

**A-9.2** Read right to left: first translate to (3, 0, 0), *then* rotate around the
origin — so the model **swings in a circle around the origin**, 3 units out, like a
door on a hinge, instead of spinning. You probably wanted it to spin in place at
(3, 0, 0): `translation(3, 0, 0) × rotation_y(angle)` — turn first (at the origin), then
move.

**A-9.3** `model = rotation_y(angle) × translation(−2, −1, 0)` — right to left: first
move the centre (2, 1, 0) onto the origin, then rotate around the origin. (To spin it in
place back at (2, 1, 0) instead: `translation(2, 1, 0) × rotation_y(angle) ×
translation(−2, −1, 0)`.) This is exactly the subject's "rotate around its centre".

**A-9.4** About **half the size** (in width and in height — a quarter of the area). The
projection matrix puts each point's distance into `w` (2 and 4), and the GPU's
**perspective divide** divides x and y by `w` after the vertex shader. Twice as far →
divided by twice as much → half as big.

**A-9.5** GLSL sees the matrix **transposed**: rows and columns swapped. The translation
(tx, ty, tz), which belongs in the 4th column, lands in the **4th row** — the row that
computes `w`. So instead of moving the model, it changes each point's `w` depending on
its position, and the perspective divide then scales points unevenly: the model is
distorted, flips, shoots off screen, or vanishes. Rotations come out turning the
opposite way. No error is reported anywhere — which is why the storage order is fixed
once and tested.

## Section 10

**A-10.1** The **camera** (`projection × view`) goes in a **uniform buffer**, reached through
a descriptor set: it changes at most once per frame and is the same for every object, so
it's written once and shared by every draw. The **model** matrix goes in a **push
constant**: it's different for every object, and push constants travel inside the command
buffer, so each draw can carry its own value at no extra cost. (Both would fit in 128
bytes of push constants, but that uses all the guaranteed space, and the descriptor
machinery is needed in § 14 anyway.)

**A-10.2** While the GPU is still reading the camera for frame N (slot A), the CPU writes
frame N+1's camera (slot B) into **the same buffer**. Frame N can then be drawn partly or
wholly with the *next* frame's camera: occasional one-frame jumps or jitter, mostly
visible when the camera moves fast. It's a race, so it may look fine on one machine and
not another. The validation layer normally reports **nothing**: every call is valid on its
own; the problem is *timing*. One buffer per slot, written after that slot's fence wait,
fixes it.

**A-10.3** The `1.0` is `w`: the corner is a **point**, so the model matrix's translation
applies to it (§ 9). Read right to left: first `push.model` places and turns the corner
in the world, then `camera.view_proj` applies the view (camera) and then the projection.
So model → view → projection, i.e. `P × V × M × corner`.

**A-10.4** A descriptor **points at** the buffer; it doesn't hold a copy of the data. The
shader reads whatever bytes are in the buffer *when the GPU runs the draw*, so changing
the buffer's contents is enough. A new descriptor write is only needed if the set must
point at a **different resource**: another buffer, a different offset or size, or (§ 14)
a different texture.

**A-10.5** The cube looks **stretched sideways** (twice as wide as it should be), because
the projection still uses the old width ÷ height. It should come from the **current
swapchain image size every frame** (`frame.aspect()`), so it's always right after a resize.

## Section 11

**A-11.1** One **depth value** per pixel: how far away (0 = near plane, 1 = far plane) the
thing currently drawn in that pixel is. It's reset to **1.0** — the far plane, "as far
as possible" — so that the first fragment drawn anywhere is always nearer (`LESS`) and
wins against the empty background.

**A-11.2** The buffer never changes from its cleared 1.0, so **every** fragment passes
the test (everything is less than 1.0) — exactly as if there were no depth buffer:
the last triangle drawn wins, and back faces show through again at some angles. The
test needs remembered depths to compare against; without writes there are none.

**A-11.3** **Only the background.** Every depth is between 0 and 1, and nothing is
*greater* than the cleared 1.0, so every fragment fails and is discarded.

**A-11.4** The depth numbers are only needed **during** the frame, to decide which
fragment wins each pixel. Once rendering ends, nothing reads them — the next frame
clears them to 1.0 anyway. The colour image is different: it's shown on screen, so it
must be kept (`STORE`). `DONT_CARE` lets the GPU skip writing the depth numbers back to
memory, which saves bandwidth.

**A-11.5** **Z-fighting.** Depth precision is crowded near the near plane; with a tiny
`near`, surfaces farther away get almost the same depth numbers, so rounding picks the
winner pixel by pixel: flickering stripes. It shows first **where two faces are close
together and far from the camera** (edges where faces meet, the far side of the model).

## Section 12

**A-12.1** **`v`** (a corner's position) and **`f`** (a face, listing corners by
number). Everything else is skipped: comments (`#`), `o` / `g` (object and group
names), `s` (smoothing), `mtllib` / `usemtl` (materials), `vt` / `vn` (texture
coordinates, normals), `l` / `p` (lines, points), blank lines.

**A-12.2** Corners 16, 2, 3, 17 → 0-based 15, 1, 2, 16. Fan from the first corner:
**(15, 1, 2)** and **(15, 2, 16)**.

**A-12.3** Every face would connect the *next* corner along instead of the right one, so
the logo turns into a tangle of wrong, spiky triangles. A face using corner 42 would ask
for `positions[42]`, but only indices 0..41 exist: a panic (index out of bounds) if
unchecked, or — with the parser's range check — a clean error with the line number.

**A-12.4** Some fan triangles can cover area **outside** the L (across the inner corner),
so the face looks like it has an extra triangle filled in, and parts may overlap. That's
fine for the mandatory files; handling concave faces properly is the § 17 bonus.

**A-12.5** `model = rotation_y(angle) × translation(−centre)` — right to left: move the
centre onto the origin, then turn. The centre is the middle of the **bounding box**:
`(min + max) / 2` of all the corners, computed by the parser.

## Section 13

**A-13.1** The greys **bleed into each other**: a shared corner can only hold one colour,
so every face touching it gets that colour at that corner, and the rasteriser blends
between corners. You'd see smooth grey gradients across the logo instead of distinct flat
faces, with no visible edges. (Which grey each corner ends up with depends on which face
wrote it last.)

**A-13.2** **18,960** vertices (6,320 triangles × 3 corners each). The indices are simply
`0, 1, 2, 3, …, 18959`: every vertex is used exactly once.

**A-13.3** Arrows are **continuous**: movement should happen every frame the key is down,
so `held`. Space is a **toggle**: one press = one switch, so `pressed` (true only on the
frame the key goes down). With `held`, the spin would flip on/off every frame while the
key is down — a random-looking on/off depending on how many frames you held it.

**A-13.4** Read right to left: centre → **move** → turn. The turn now happens *after* the
move, so the position itself gets turned: `rotation_y(90°)` turns +x into −z. Holding →
increases position.x, which comes out as moving **away from the camera** (the model gets
smaller) instead of right — and changing the angle swings the model around the origin.
The translation card must be leftmost for screen-axis (world-space) movement.

**A-13.5** **`dt`**: per second, so the same speed at any frame rate. **`radius`**: in the
model's own size, so a tiny logo and a big teapot both move at a sensible pace.
**`0.2`**: the chosen speed — a fifth of the model's size per second.

## Section 14

**A-14.1** Two numbers per corner saying **where on the picture** that corner sits: **U**
across (0 = left edge, 1 = right edge) and **V** down (0 = top, 1 = bottom). The GPU blends
them across each triangle so every pixel knows its spot, then looks the colour up there.
scop **makes them up** with planar mapping: project along the bounding box's thinnest axis
and scale the other two coordinates into 0..1.

**A-14.2** **Long streaks** of the picture's edge colours. Along a thin side face, the
projected axis (x) changes but U and V barely do — every point on that face maps to
almost the same thin strip of the picture, so those few texels get smeared across the
whole face. That's the "visible stretching" the § 18 bonus asks you to remove.

**A-14.3** The picture is **upside down** (mirrored top to bottom): the loader treats the
first stored row — the picture's bottom — as the top.

**A-14.4** Written by the CPU **once**, read by the GPU every frame → staging into
`DEVICE_LOCAL` (fastest reads), same as the meshes. `UNDEFINED → TRANSFER_DST_OPTIMAL`:
make the image ready to **be written by the copy**. `TRANSFER_DST_OPTIMAL →
SHADER_READ_ONLY_OPTIMAL`: after the copy, make it ready (and the copy's writes visible)
for **fragment shader reads**.

**A-14.5** **Washed out / too pale.** The file's bytes are already sRGB-encoded (extra
detail in dark shades). `UNORM` tells the GPU they're plain brightness, so it skips the
decoding; the sRGB swapchain then encodes them a *second* time on output. Double-encoded
= everything brighter and paler than the picture really is.

## Section 15

**A-15.1** `t = 0` → **`a`**; `t = 0.5` → **halfway between** `a` and `b`; `t = 1` → **`b`**.
In scop: `a` = the face's grey, `b` = the picture's colour at that spot, `t` = the blend
factor (eased progress of the fade).

**A-15.2** So the fade can **move gradually** and **reverse without a jump**. The key only
says which way to go; each frame the blend factor takes a small `dt`-sized step towards
that target. Setting it directly would snap the picture in or out — the abrupt cut the
subject forbids.

**A-15.3** Progress stops climbing at 0.4 and starts heading back down from **0.4** — no
jump — reaching 0 (all grey) about 0.4 × 0.6 s ≈ 0.24 s later, then staying there. The
picture fades out smoothly from where it was.

**A-15.4** A **validation error when the pipeline is created**: the fragment shader uses
push-constant bytes that no range declares for the `FRAGMENT` stage. (If you ran anyway,
the blend value the fragment shader reads would be undefined.)

**A-15.5** The **pace** differs. Linear moves at a constant speed, so it starts and stops
with a sudden "kick". smoothstep starts slowly, speeds up in the middle, and slows down
again at the end, so you see the difference at the **very beginning and very end** of the
fade — it eases in and out instead of starting and stopping abruptly.

## Section 16

**A-16.1** The swapchain's images are made for **one window size**; after a resize they no
longer fit, and acquire/present report out-of-date (or suboptimal). The **depth image**
must be rebuilt with it, because it has to be exactly the swapchain's size (one depth slot
per pixel). The pipelines don't need rebuilding because viewport and scissor are
**dynamic state**, set per frame from the current extent (§ 6) — and the formats don't change.

**A-16.2** The GPU may still be using the old swapchain's images, views and semaphores for
frames already submitted. Destroying them under it is a **validation error** (object in
use) and can crash or lose the device. `device_wait_idle` first guarantees nothing is in use.

**A-16.3** It checks for **`cargo`/Rust**, **`glslc`** and the **Vulkan loader** (and MoltenVK on
macOS). Rust can always be installed into your home folder (rustup, no admin). On campus
Linux, `glslc` and Vulkan are **system packages that need admin rights**, so the script
can't install them — it reports exactly what's missing and the command to get it. The
subject says *check for and possibly install*: checking is required, installing only
where possible.

**A-16.4** (1) First line in italics: *This project has been created as part of the 42
curriculum by luferna3.* (2) A **Description** section (goal + brief overview).
(3) An **Instructions** section (compilation, installation, execution). (4) A **Resources**
section with classic references **and** how AI was used — for which tasks and which parts.

**A-16.5** `vt` and `vn` lines are **skipped** (scop makes its own UVs and doesn't light).
`f 1/1/1 2/2/2 3/3/3` uses only the part before the first `/`, so it's corners 1, 2, 3
→ 0-based 0, 1, 2. The 5-corner face is **fan-triangulated** into 3 triangles, all with the
same grey (one face number) — correct if it's convex; a concave one may show stray
triangles (§ 17 bonus).

## Section 17

**A-17.1** An **ear** is three consecutive corners whose triangle lies entirely **inside** the
polygon: the middle corner points outwards (convex turn) and no other corner is inside the
triangle. Cutting it off removes only area that belongs to the face, and what's left is still a
valid polygon with one corner fewer — so repeating always ends in a correct triangulation.

**A-17.2** The **area check**: for a flat face, a correct triangulation's triangle areas add up to
exactly the face's area. A fan from a bad corner of an L includes a triangle that reaches
outside the shape, so its areas add up to **more** than the face's area (and overlap).

**A-17.3** The ear tests ("does it turn outwards?", "is a corner inside this triangle?") are
simple and reliable in **2D**; flattening along the face's normal keeps its outline and
turning direction. **Newell's normal** uses every corner, so it gives the true average facing
even for concave or bent faces; the cross product of the first two edges only sees three
corners — if the middle one is a **reflex** corner, it points the *wrong way* and every test
would be inverted.

**A-17.4** It stops looking for ears and **fans the remaining corners** (the fallback). That
guarantees the loop ends (no hang on a broken file) and still draws something reasonable for
that face, while the rest of the model loads normally — better than refusing a whole model
over one malformed face.

**A-17.5** Show it's **in the file**: both `teapot.obj` and `teapot2.obj` render the same rings
from the same angle, and counting edges shows ~100 **open edges** around the lid rim in both
files (edges used by only one triangle — the lid and body surfaces never join). scop draws
exactly what the file describes, from both sides, with correct depth.

## Section 18

**A-18.1** § 14 projected the picture along **one** axis for the whole model. Faces running
*along* that axis (the logo's thin sides) are edge-on to the projection: every point on them maps
to almost the same thin strip of the picture, which gets smeared across the face. Box mapping
gives **each face** the projection it faces most directly (by its Newell normal), so no face is
ever edge-on to its projector.

**A-18.2** Dividing each by its own side squeezes the picture to fit the box both ways: on a long
thin object, one direction gets many more texels per unit than the other, so the picture
**stretches**. One shared scale keeps the same texel density across and down — squares stay square.

**A-18.3** Which picture a draw reads is decided by the **texture set bound for that draw** (set 1,
§ 14). One mesh drawn once can only have one set bound, so one picture. Splitting the model into
one mesh per object, each drawn with its own `draw_textured`, lets each bind its own picture.

**A-18.4** A **validation error when the pipeline is created**: `POLYGON_MODE_LINE` requires the
`fillModeNonSolid` feature to be **enabled on the device**, not just supported by the GPU. (Without
validation, the result is undefined — maybe solid triangles, maybe a crash.)

**A-18.5** The **perspective** differs. Moving closer exaggerates it: parts nearer the camera grow
much more than parts further away, so the model looks deeper and more distorted. Zooming keeps the
camera where it is and only crops the view, so near and far parts grow equally: the model looks
**flatter**, like a telephoto shot. (Same size on screen, different shape.)

## Section 19

**A-19.1** The **vsync cap**: scop presents with FIFO, which waits for the screen's refresh, so
it can't show more frames than the screen draws (120 Hz → 120 fps). The number tells you scop
keeps up with the screen; it does **not** tell you how fast the GPU could go, or which model
is heavier. For that, compare frame times in a release build (or a present mode without the cap).

**A-19.2** (1) `1 ÷ dt` changes every frame — tiny timing jitter makes it flicker between values
you can't read. An average over 0.5 s is steady and closer to the truth. (2) The title would be
set every frame, which is slow on some systems (macOS redraws the title bar); averaging means it
changes twice a second, and only when the text differs.

**A-19.3** Up to two frames are still in flight (§ 5), and they were recorded with the old
model's buffers. Destroying them now means the GPU may read freed memory while finishing those
frames: a **use-after-free on the GPU**, in the next few milliseconds after the drop. With
validation on, yes: it reports a buffer **destroyed while still in use** by a submitted command
buffer. Without validation, it's undefined — garbage, a crash, or nothing visible. Fix: wait
(`device_wait_idle`) before destroying.

**A-19.4** (1) The file is **inside out**: its faces list their corners clockwise seen from
outside. (2) scop's **front/back is swapped**: the pipeline's `front_face` doesn't match the
winding after the projection's y flip. Test: open a model you trust (the teapot or the cube) — if
it's red outside too, the cause is scop's setting, not the file.

**A-19.5** Stored depth is **not linear**: the perspective card squeezes far distances towards
1, keeping most precision near the near plane. scop's near plane is tiny (1 % of the model's
size), so the whole model lands around 0.99–1.0: almost the same white everywhere. scop uses the
**linear distance** — `w` from the vertex shader — mapped over the model's own nearest–farthest
range, so the full white-to-dark scale is used.

## Section 20

**A-20.1** Orbiting moves only the camera: the model and the light stay where they are, so the
**same faces stay lit** — you walk round and see the lit side, then the dark side. Turning the model
moves its faces relative to the light, so **which faces are lit changes** as it turns.

**A-20.2** The camera **flips upside down** in one frame as it passes straight up: "up" (+y) and
the view direction become the same line, so which way is up stops being defined, and on the far
side it comes out reversed. The clamp (±89°) stops it just before.

**A-20.3** The light looks **painted on**: the faces that were bright stay bright and the dark ones
stay dark, however the model spins — the shader still thinks every face points where it did at
the start.

**A-20.4** Faces turned away from the light would get **zero** — pure black, invisible against the
dark background, and the model's outline would vanish on its dark side. Ambient keeps every face
at least dimly visible.

**A-20.5** A corner holds **one** normal. With its own corners, each triangle's corners carry its
face's normal, so the whole face has one brightness and edges stay crisp. Shared corners belong to
several faces at once, so they'd need one **averaged** normal — brightness would blend across
edges: **smooth shading**, rounded-looking, wrong for the logo's sharp corners.

## Section 21

**A-21.1** Vulkan only accepts **SPIR-V** (§ 6), never GLSL text. Something has to translate the
`.frag` into SPIR-V first — at build time that's `build.rs`; for hot reload it's the same `glslc`,
run while scop is running.

**A-21.2** So a failure costs nothing: if the new shader doesn't compile or the pipeline can't be
built, the **old pipelines are still there** and scop keeps drawing with them. Destroying first would
leave scop with nothing to draw with (the same rule as dropping a model in § 19).

**A-21.3** **Screen: unchanged** — the old shaders keep running. **Terminal: `glslc`'s error**, with the
file and line of the problem. Fix and save → the next check sees a newer modification time, compiles,
swaps, and prints "shaders reloaded".

**A-21.4** It breaks the **shader contract**: the shader now expects a vertex input at location 4, but
the Rust side's vertex layout (§ 7) has nothing there — validation reports the missing input (without
validation, the values are undefined). Hot reload can't add data the Rust side doesn't send: it needs
a `tangent` field in `Vertex`, a 5th attribute, the data filled in by scop — and a rebuild and restart.

**A-21.5** A pipeline id means "scop's solid (wireframe, points) pipeline", not "this exact version":
scop *wants* every later draw to use the new one, without updating its tickets. Nothing can draw the
old one by mistake — it's destroyed only after the GPU finished with it, and nobody holds a separate id
for it. In § 19 the slot couldn't be reused because a *different* mesh could have appeared under an old
id: wrong shape, no error. Here it's the same thing, updated.

## Section 22

**A-22.1** It saves the **repeating**: recording 512 sets of bind / push / draw commands on the CPU,
and the GPU's start-up cost for 512 draws — it's one command saying "512 copies". It does **not**
save the drawing: the GPU still runs the vertex shader for every corner of every copy and fills
every triangle (512 × the mesh).

**A-22.2** **Too big for the push block**: it holds about one matrix (128 bytes guaranteed), and 512
matrices are 32 KB — a buffer the shader reads like an array. **One per frame slot** because the CPU
rewrites the list every frame while the GPU may still be reading last frame's (§ 10): a shared list
would make copies jump or flicker, silently.

**A-22.3** It's drawn with the **first crowd copy's matrix** instead of the identity: shrunk to the
size of one copy, moved to a corner of the grid, turning on its own — because ordinary draws read
position 0, and position 0 is no longer "change nothing".

**A-22.4** No. The **vsync cap** (§ 19): FIFO waits for the screen, so the title can't show more than
60 whether the frame took 2 ms or 15 ms of real work. The GPU is drawing 512 × the teapot's triangles;
to see the cost, compare frame times without the cap, or watch for the number dropping *below* 60.

**A-22.5** The vertex shader turns each normal by the **whole** model for that copy — sticky note ×
the copy's own matrix — the same combined card that places its corners. So each copy's normals turn
with it, and the unchanged light code (§ 20) sees the right direction for every face of every copy.

## Section 23

**A-23.1** Six square pictures — the inside faces of a box — stored as one texture. It's looked up with a
**direction** because it describes what's *around* a point, not across a flat surface: the direction
says "what's over there?", and the GPU picks the face it points at and the spot on it. Every direction
has an answer, so it covers the whole surroundings.

**A-23.2** It would behave like a room, not a sky: as the camera moves on its orbit it gets nearer one
wall and further from another, walls slide past at different speeds, and you could see the box's
**edges and corners**. Moving with the camera, only turning changes the view — like something infinitely
far away.

**A-23.3** **Only the sky.** Drawn first with depth writing on, the box writes its (very near) depth over
the whole screen; the model is further away, so it fails the depth test (§ 11) everywhere and is never
drawn. With depth writing off, the box paints the background but leaves the depth image at "nothing
here", so the model passes the test and draws over it — whatever the box's size.

**A-23.4** Each face is **flat** with one normal (flat shading, § 20), so each acts as its own flat mirror.
Neighbouring faces point different ways, so at their shared edge the bounce direction jumps — the
reflection breaks sharply. The teapot's hundreds of small flat faces become hundreds of little mirrors:
a disco ball. Smooth (averaged) normals would give a smooth chrome.

**A-23.5** So everything tells the same story: the model's bright side (§ 20 light) faces the sun you see
in the sky and in its reflections. With the sun elsewhere, the model would be lit from one side while
the sky's sun sits on another — it looks fake immediately, even if you can't say why.

## Section 24

**A-24.1** **scop.** Another project wouldn't want grey faces unchanged — they exist because the subject
asks for distinguishable greys: a choice, not a mechanism. What goes to the engine is the *mechanism*:
building one mesh per object with its own corners and flat normals, with the colour asked for through a
**callback** — scop's callback returns its greys.

**A-24.2** Because the flip comes from the engine's own camera maths (`look_at` with a fixed "up"): it's a
property of the engine's camera, not of an app. If each app had to remember the clamp, some would forget
and get the snap upside down over the top. The engine owns the guard (as a setting with a safe default).

**A-24.3** Write your own shaders (an **override**), make a pipeline from them, and draw with `draw_with`,
whose four numbers mean whatever your shaders say. Reusing the default pipeline would feed your numbers
into shaders that read them as blend / colour mode / depth range — wrong colours, silently. Each pipeline
owns the meaning of its push params.

**A-24.4** Only the app knows which of its pipelines were built from which shaders, in which style (fill,
wireframe, points, background). The engine just sees pipelines and bytes. The general half — noticing a
save and compiling — is the same for every project, so it moves; the mapping stays with the app.

**A-24.5** The camera's "up" is hard-coded as +y: in a +z-up world, orbiting climbs the wrong way, the
horizon tilts, and the pitch clamp guards the wrong axis. The **up axis** (and the clamp relative to it)
should be a setting, with +y as the default so scop doesn't change.
