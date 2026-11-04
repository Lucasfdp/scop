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
