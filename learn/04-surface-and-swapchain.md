---
section: 4
slug: surface-and-swapchain
tier: T2
status: not-started
prereqs: [3]
terms: [surface, VK_KHR_surface, VK_EXT_metal_surface, raw-window-handle, ash-window, present, VK_KHR_swapchain, surface capabilities, surface format, sRGB, present mode, FIFO, MAILBOX, extent, VkImage, image view, ERROR_OUT_OF_DATE_KHR]
---

# Section 4 — Getting images to the screen

**Depth: T2** — the swapchain is where Vulkan meets the window, and it's the part that
breaks when the window resizes, minimises or moves to a Retina screen. Understanding
each setting here is what lets you fix those bugs in seconds rather than hours.
**You'll be able to:** connect Vulkan to your `winit` window, create the set of images
that take turns on screen, and explain every choice you made while creating them.

---

## The idea

After § 3 you can talk to the GPU, but the GPU has no idea your window exists. Vulkan
deliberately knows nothing about windows — it runs on servers with no screen, on
phones, on consoles — so you need an adaptor between "a window on macOS" and "a place
Vulkan can put finished images".

Think of a theatre with a stage (the window) and a small set of identical picture
frames (the images). While the audience looks at the frame on stage, painters behind
the curtain paint the next frame. When a painting is finished it's handed to the stage
manager, who swaps it on stage at the right moment — never halfway through the
audience's view — and hands the previous frame back to be painted over.

- The **adaptor** to the window is the *surface*.
- The set of frames, with the rules for swapping them, is the *swapchain*.
- The stage manager is the system's *presentation engine* — macOS's compositor, not
  your code. You only ask "give me a free frame" and "please put this one on stage".

To create the set of frames you must answer, in advance: how big are the frames? What
colour format? How many of them? And what's the swapping rule — wait for the screen's
refresh, or swap the moment a frame is ready? You don't get to invent the answers: you
ask the surface what it supports, then choose from that menu.

**Where the theatre analogy stops being true:** painters and stage manager work at the
same time, with no shouting between them. Coordinating "this frame is finished" and
"this frame is free again" is done with signals, which is § 5.

---

## The names for it

**surface** — `VkSurfaceKHR`: Vulkan's handle for "something I can present images to",
here your window. Created from the instance; needs the window to stay alive as long
as it exists.

**`VK_KHR_surface`** — the instance extension that defines surfaces in general.

**`VK_EXT_metal_surface`** — the instance extension that creates a surface from a macOS
*Metal layer* (the drawable area of a window, `CAMetalLayer`). Each platform has its
own: `VK_KHR_xlib_surface`, `VK_KHR_wayland_surface`, `VK_KHR_win32_surface`… That's
why you won't hard-code them.

**`raw-window-handle`** — a tiny crate that defines a common format for "here is a
window handle, from some windowing library, for some platform". `winit` produces one;
Vulkan code consumes one. Nothing more.

**`ash-window`** — a small companion crate to `ash` that reads a `raw-window-handle`
and (a) tells you which surface instance extensions this platform needs, and (b)
creates the right kind of surface. Window glue only — no rendering logic. Logged in
`DECISIONS.md`.

**present** — to hand a finished image to the presentation engine to be shown.
"Presentation" is Vulkan's word for "putting it on screen".

**`VK_KHR_swapchain`** — the device extension that provides swapchains. You add it when
creating the logical device (§ 3's step 5) — and check the physical device supports it.

**surface capabilities** — what the surface allows: minimum and maximum image count,
current size, minimum and maximum size, supported transforms and alpha modes. You
query it; you don't set it.

**surface format** — the pixel format (how many bits per colour channel, in what order)
plus the colour space. Typically `B8G8R8A8_SRGB`: **B**lue, **G**reen, **R**ed, **A**lpha,
8 bits each, sRGB-encoded.

**sRGB** — **s**tandard **R**ed **G**reen **B**lue. A colour encoding that spends more of
its precision on dark shades, because human eyes are more sensitive there. With an
`_SRGB` format the GPU converts automatically when writing: your shader works in
"linear" values (0.5 = half the light), the image stores them sRGB-encoded. Matters
for your "subtle shades of grey": the same number looks different in the two encodings.

**present mode** — the swapping rule.

**FIFO** — **F**irst **I**n, **F**irst **O**ut: finished images queue up and one is shown
per screen refresh. This is vsync: no tearing, capped at the display's refresh rate.
**The only mode Vulkan guarantees every platform supports.**

**MAILBOX** — like FIFO, but the queue holds only one image: a newer finished image
*replaces* the waiting one, like a letterbox that only fits the latest letter. Lower
delay, no tearing, but the GPU keeps working flat out. Optional — may not be offered.

**extent** — Vulkan's word for a 2D size in pixels: `width × height`.

**`VkImage`** — a block of GPU memory that holds pixels, plus a description (size,
format, usage). The swapchain's images are `VkImage`s you **don't own** — the swapchain
created them and destroys them.

**image view** — `VkImageView`: describes *how to look at* an image — which format to
read it as, which part of it (mip levels, layers). Shaders and rendering never touch a
`VkImage` directly; they go through a view. You create one per swapchain image, and
you *do* own those.

**`ERROR_OUT_OF_DATE_KHR`** — the result you get when the swapchain no longer matches
the surface (the window was resized). Its sibling `SUBOPTIMAL_KHR` means "still works,
but no longer ideal". Both mean: rebuild the swapchain (§ 16).

---

## How it actually works

### Where it fits

```
              § 3                                   § 4
 Entry ─► Instance ─────────────► Surface (from the winit window)
             │   (+ surface extensions)    │
             ▼                             │ "can this queue family present here?"
      Physical device ◄───────────────────┘
             │
             ▼
      Logical device (+ VK_KHR_swapchain) ─► Swapchain ─► its VkImages ─► your image views
```

This changes the order from § 3: the instance now needs the window's surface
extensions, and choosing the physical device now needs the surface. So **Vulkan setup
has to happen after the window exists** — in `winit`'s `resumed`, right after the
window is created.

### 1. Instance extensions for the window

`ash_window::enumerate_required_extensions(display_handle)` returns the list for this
platform (on macOS: `VK_KHR_surface` + `VK_EXT_metal_surface`). Add them to § 3's list.

### 2. Creating the surface

`ash_window::create_surface(...)` takes the entry, the instance, and the window's
display and window handles (`winit`'s window implements the `raw-window-handle`
traits). To query and destroy surfaces you also create `ash::khr::surface::Instance` —
the function table for the surface extension.

### 3. Choosing a device that can present

A queue family that can draw can't necessarily present to *this* surface. For each
candidate family, ask `get_physical_device_surface_support(device, family, surface)`.
Prefer one family that does both (on Apple GPUs, family 0 normally does). Also check
the device supports `VK_KHR_swapchain`.

### 4. Asking the surface what's on the menu

Three queries:
- **capabilities** → image count limits, current extent, transforms, alpha modes
- **formats** → list of (format, colour space) pairs
- **present modes** → list of modes

### 5. Choosing

| Setting | Choice | Why |
|---|---|---|
| Format | `B8G8R8A8_SRGB` + `SRGB_NONLINEAR` if offered, else the first one | Correct gamma for free |
| Present mode | `FIFO` (or `MAILBOX` if offered and you want lower latency) | FIFO is always available; vsync is fine for scop |
| Extent | `current_extent` — unless it's the special value `u32::MAX`, which means "you choose": then use the window's **physical** size, clamped to min/max | Must match the window's real pixels |
| Image count | `min_image_count + 1`, clamped to `max_image_count` (**0 means no maximum**) | One spare so you're not waiting on the presentation engine |
| Usage | `COLOR_ATTACHMENT` | You'll render into them |
| Sharing | `EXCLUSIVE` | One queue family uses them |
| Transform | `current_transform` | No rotation |
| Composite alpha | `OPAQUE` if supported, else the first supported | The window isn't see-through |
| Clipped | `true` | Don't draw pixels hidden by other windows |

Then create the swapchain through `ash::khr::swapchain::Device` (the swapchain
extension's function table), and fetch its images.

### 6. Image views

For each swapchain image: a 2D view, same format as the swapchain, the colour
"aspect", one mip level, one layer. Store them next to the swapchain.

### 7. Ownership and destruction

New order, mirror of creation:

**image views → swapchain → device → surface → debug messenger → instance**

And one Rust-specific rule: **the window must outlive the surface.** If the window is
dropped first, the surface points at a dead window. Keep the GPU context and the window
in the same struct, with an explicit `Drop` (or field order) that drops the GPU context
first.

### Retina screens

Your Mac's screen has 2 physical pixels per "logical" point. You asked `winit` for a
1280×720 *logical* window; its physical size is 2560×1440. Vulkan works in physical
pixels: use `window.inner_size()` (physical), never the logical size, for the extent.

---

## Known traps

- **Everything renders in the bottom-left quarter, or blurry** — you used the logical
  size on a Retina screen. Use the physical size.
- **Crash or validation error when minimising** — the window's size becomes 0×0, and a
  0×0 swapchain is invalid. Skip rendering (and rebuilding) while the size is zero.
- **`ERROR_OUT_OF_DATE_KHR` after resizing** — expected; you'll rebuild in § 16. Until
  then, resizing may stop rendering. Note it, don't panic.
- **Validation error: swapchain extension not enabled** — you forgot `VK_KHR_swapchain`
  in the device extensions.
- **"max_image_count is 0, so I asked for 0 images"** — 0 means *unlimited*, not
  "zero allowed". Only clamp when it's non-zero.
- **Destroying swapchain images yourself** — you don't own them. Destroy the views,
  then the swapchain; the swapchain cleans up its images.
- **Window dropped before the surface** — use-after-free across the Rust/C boundary;
  validation may not catch it. Drop order matters.
- **Greys look washed out or too dark** — a mismatch between an `_SRGB` and a `_UNORM`
  format and how you think about colour values. Know which one you chose.

---

## Checkpoint

**Satisfies:** "inside a window" at the Vulkan level — the window now has images
Vulkan can draw into. (Method: gate first, then Claude writes it and walks you through it.)

1. `ash-window` (and `raw-window-handle` if needed) added to the engine, pinned.
2. Vulkan setup moved into `resumed`, after the window is created; instance gets the
   surface extensions; surface created from the window.
3. Physical device choice also requires present support and `VK_KHR_swapchain`.
4. Swapchain created with the choices in the table; image views created.
5. Print: chosen format, present mode, extent, image count.
6. Clean destruction in the new order; the window outlives the surface.

**Verify:** window opens; the printout shows a sensible format, `FIFO` (or `MAILBOX`),
an extent about twice the logical size, and 2–3 images; closing gives no validation
messages. (The window is still blank or garbage — nothing draws until § 5.)

---

## Questions

Q-4.1 (recall) What is the surface for, and why doesn't core Vulkan just know about
windows?

Q-4.2 (application) The surface reports `min_image_count = 2` and
`max_image_count = 0`. How many images do you ask for, and why isn't 0 a problem?

Q-4.3 (prediction) You create the swapchain with the extent 1280×720 because that's
what you passed to `winit`, on your Retina MacBook. What do you see once rendering
works, and what should you have used?

Q-4.4 (prediction) The user minimises the window, and your code rebuilds the swapchain
on every resize event. What goes wrong?

Q-4.5 (prediction) In § 3 Vulkan setup ran before the window opened. Why can't it any
more? Name the two steps that now need the window.

Answers: `ANSWERS.md` § 4
