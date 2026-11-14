---
section: 11
slug: depth-buffer
tier: T2
status: done
prereqs: [10]
terms: [occlusion, painter's algorithm, depth test, depth buffer, depth value, compare op, depth write, depth clear value, depth image, depth format, format features, image aspect, depth attachment, depth layout, store op DONT_CARE, early and late fragment tests, stencil, z-fighting, back-face culling, winding order]
---

# Section 11 — What's in front of what

**Depth: T2** — every 3D thing you'll ever draw needs this, and when it's wrong the
model looks inside-out with no error message. You should be able to say what the depth
buffer holds, why each setting has the value it has, and diagnose the classic failures
from a screenshot.
**You'll be able to:** draw the cube (and later the 42 logo and the teapot) with the
near faces always covering the far ones, at every angle.

---

## The idea

Watch your § 10 cube for a few seconds: at some angles a face from the *back* shows
through on top of a front face. Nothing is wrong with the matrices. The GPU simply
draws triangles **in the order of the index list**, and each new triangle paints over
whatever pixels are already there. Whoever is drawn **last** wins, whether it's near or
far.

The obvious fix is to paint like a painter: sort the triangles far to near and draw the
far ones first, so near ones cover them. It half-works, but it costs a sort every frame
(the cube turns, so the order keeps changing), and it **can't** handle triangles that
cut through each other: each is partly in front of the other, so no order is right.

The fix every GPU uses is much simpler: **remember, for every pixel, how far away the
thing currently drawn there is.** When a new triangle wants to paint a pixel, compare:

- new one is **nearer** than what's there → paint it, and remember the new distance;
- new one is **farther** → throw it away, the pixel keeps what it had.

Now the order triangles are drawn in no longer matters. Picture a sheet of paper the
size of the window where, in pencil, you've written a distance in every square.
Before each frame you rub it all out and write "as far as possible" everywhere, so the
first thing drawn in any square always wins against "nothing".

You've already met the distance number: it's the **depth** from § 9's projection, where
the near plane comes out as 0 and the far plane as 1. "As far as possible" is 1.

**Where the analogy stops being true:** the sheet isn't checked by your code at all.
You switch the check on in the pipeline, give the GPU an image to keep the numbers in,
and the GPU does the comparison itself, for every pixel of every triangle, in hardware.

---

## The names for it

**occlusion** — one thing hiding another from view. "Correct occlusion" = near things
cover far things.

**painter's algorithm** — drawing far-to-near so near things paint over far ones.
Named after how painters do backgrounds first. Fails for intersecting triangles and
needs re-sorting whenever anything moves. Not used in scop.

**depth test** — the per-pixel comparison: "is this new fragment nearer than what's
there?" A *fragment* (§ 6) is a candidate pixel produced by one triangle. Fails → the
fragment is thrown away before it writes any colour.

**depth buffer** — the "sheet of paper": one number per pixel, the depth of whatever is
currently drawn there. Also called the **z-buffer**, because depth comes from z.

**depth value** — the number stored: § 9's `z / w` after the perspective divide. 0 at the
near plane, 1 at the far plane.

**compare op** — `VkCompareOp`, "comparison operation": the rule for "nearer". scop uses
`LESS`: the new fragment passes if its depth is **less than** the stored one (smaller =
nearer).

**depth write** — after a fragment passes, store its depth in the buffer. A separate
switch from the test: you can test without writing (used for see-through things, later).

**depth clear value** — what every pixel is reset to at the start of each frame: **1.0**,
the far plane, so the first fragment anywhere always passes `LESS`.

**depth image** — the depth buffer as a Vulkan object: a `VkImage` the size of the
swapchain images, in `DEVICE_LOCAL` memory (only the GPU touches it), plus a
`VkImageView` (§ 4) so rendering can use it.

**depth format** — how each depth number is stored. scop asks for `D32_SFLOAT`:
**D**epth, **32** bits, **S**igned **FLOAT**. Same idea as § 10's `R32G32B32_SFLOAT`, with
**D** for depth instead of colour channels.

**format features** — what a GPU can do with a format, asked with
`get_physical_device_format_properties`. You need the feature
`DEPTH_STENCIL_ATTACHMENT` ("usable as a depth/stencil attachment") for optimal tiling,
or the format isn't allowed. scop checks a short list and takes the first supported one.

**image aspect** — which part of an image a view or barrier talks about: `COLOR` for
colour images (§ 5's `COLOUR_RANGE`), `DEPTH` for depth images. Using `COLOR` on a depth
image is a validation error.

**depth attachment** — the depth image plugged into rendering. With dynamic rendering
(§ 6 decision) it's one more `RenderingAttachmentInfo`, passed as `depth_attachment`
next to the colour attachment.

**depth layout** — the layout (§ 5) an image must be in to be used as a depth
attachment: `DEPTH_STENCIL_ATTACHMENT_OPTIMAL`. One barrier per frame moves it there,
like the colour image's barrier.

**store op DONT_CARE** — "don't care", for the store op (§ 6): after rendering, the GPU
may throw the depth numbers away. Nobody reads them after the frame, so keeping them
would only cost memory bandwidth. The colour image uses `STORE`, because it's shown.

**early and late fragment tests** — the two pipeline stages where depth testing can
happen: *early* = before the fragment shader runs (fast: hidden fragments skip the
shader entirely), *late* = after it. A barrier on the depth image names both stages.

**stencil** — a second per-pixel number some depth formats carry (the `S8` in
`D32_SFLOAT_S8_UINT`), used for masking effects like outlines and mirrors. Named after
paint stencils. scop doesn't use it; it only explains why the layout name says
`DEPTH_STENCIL`.

**z-fighting** — two surfaces at almost the same depth, so rounding decides the winner
pixel by pixel and frame by frame: a flickering stripy pattern. Made worse by a tiny
near plane (below).

**back-face culling** — skipping triangles that face away from the camera, before
they're drawn. A speed-up, **not** a replacement for the depth buffer (a near face can
still hide another front-facing face). Off in scop — see the decision.

**winding order** — whether a triangle's corners go round clockwise or counter-clockwise
as seen on screen. Culling uses it to decide "facing me" vs "facing away".

---

## How it actually works

### 1. The depth image

At startup, next to the swapchain (it must be the same size as the swapchain images):

```
1. pick a format:    first of [D32_SFLOAT, D32_SFLOAT_S8_UINT, D24_UNORM_S8_UINT]
                     whose optimal-tiling features include DEPTH_STENCIL_ATTACHMENT
2. create_image:     2D, swapchain extent, that format, usage DEPTH_STENCIL_ATTACHMENT
3. memory:           requirements → DEVICE_LOCAL type → allocate → bind (same as § 7's Buffer)
4. create_image_view: aspect DEPTH
```

It's the image version of § 7's `Buffer`: object, memory, bind. One depth image, not
one per frame slot — see the decision and § 3 below.

When the window resizes, the depth image must be rebuilt with the swapchain (§ 16).

### 2. The pipeline switches it on

Two additions to pipeline creation:

```
depth/stencil state:  depth_test_enable  = true
                      depth_write_enable = true
                      depth_compare_op   = LESS
rendering info:       depth_attachment_format = the chosen format
```

The second line is dynamic rendering's version of "this pipeline draws into a depth
image of this format". It must match the image you actually attach, or validation
complains.

### 3. Every frame

```
barrier:  depth image  UNDEFINED → DEPTH_STENCIL_ATTACHMENT_OPTIMAL
          wait for: last frame's depth writes   (LATE_FRAGMENT_TESTS, DEPTH…_WRITE)
          before:   this frame's depth tests    (EARLY | LATE_FRAGMENT_TESTS, READ | WRITE)
begin rendering:
          colour attachment  (unchanged)
          depth attachment   view = depth view, layout DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                             load CLEAR to 1.0, store DONT_CARE
… draws unchanged …
```

Why `UNDEFINED` as the old layout: "I don't care what's in it", because it's about to be
cleared anyway. Same trick as the colour image (§ 5).

Why the barrier waits for the *previous* frame's depth writes: with two frames in
flight, the GPU might still be finishing frame N's depth tests when frame N+1 starts
using the same image. The barrier makes N+1's depth work wait for N's. Both frames run
on the same queue, so a barrier is enough here; no semaphore needed (see the sync
cheat sheet: "later commands wait for earlier ones").

### 4. What the GPU does per fragment

```
fragment at pixel (x, y) with depth d
   d < depth_buffer[x, y] ?      (compare op LESS)
     yes → run/keep the colour, depth_buffer[x, y] = d     (depth write)
     no  → discard
```

When it can, the GPU does this **before** the fragment shader (early tests), so hidden
pixels cost almost nothing.

### 5. Precision, in one paragraph

Depth after the perspective divide is **not** spread evenly: most of the 0..1 range is
spent close to the near plane. Moving the near plane closer (0.1 → 0.0001) squeezes the
rest of the scene into a tiny slice of numbers, and far surfaces start z-fighting. Rule
of thumb: keep `near` as large as you can get away with. scop's 0.1 is fine.

### Decision: one depth image + a barrier; culling off

| Option | For | Against |
|---|---|---|
| **One depth image, barrier each frame** ✅ | Standard; half the memory; the barrier is one more of what § 5 already does | Frames in flight are serialised on depth (tiny cost; they run on one queue anyway) |
| One depth image per frame slot | No barrier between frames | Twice the memory, more objects to rebuild on resize (§ 16), for no visible gain |
| **Back-face culling off** ✅ | Correct for any `.obj`, whatever its winding | Draws hidden back faces too (the depth test still hides them) |
| Back-face culling on | Up to ~2× fewer fragments | `.obj` files don't guarantee consistent winding; a model with mixed winding loses faces. Revisit for ft_vox, where you make the meshes yourself |

---

## Known traps

- **Back faces still show through, no error** — the depth image is attached but the
  pipeline's `depth_test_enable` is off, or `depth_write_enable` is off (Q-11.2).
- **Only the background, nothing drawn** — compare op `GREATER` (or clear value 0.0 with
  `LESS`): nothing can beat the cleared value (Q-11.3).
- **Validation error at draw time about the depth attachment format** — the pipeline's
  `depth_attachment_format` doesn't match the attached image's format (or is missing).
- **Validation error about image aspect** — a view or barrier on the depth image uses
  `COLOR` instead of `DEPTH`.
- **Validation error about layout** — the barrier is missing, or rendering names a
  different layout than the barrier moved it to.
- **Flickering stripes where two faces nearly touch** — z-fighting. Check the near plane
  isn't tiny (Q-11.5).
- **Whole model drawn inside-out (you see the inside of the far faces)** — the depth
  test is fine; back-face culling is on with the wrong `front_face`. Leave culling off.
- **Validation error at shutdown: image / image view / memory not destroyed** — the
  depth image's three objects go after `device_wait_idle`, before the device.

---

## Checkpoint

**Satisfies (subject):** a correctly drawn 3D object from any angle — needed before the
logo appears in § 12. Method: gate first, then Claude writes it and walks you through it.

1. **Engine — depth image:** a small `DepthImage` (image + memory + view), created with
   the swapchain, format chosen from the list by its features, `DEVICE_LOCAL`,
   destroyed in the right order.
2. **Engine — pipeline:** depth test + write on, `LESS`, and the depth format in the
   rendering create info.
3. **Engine — each frame:** the depth barrier, then a depth attachment cleared to 1.0
   with store `DONT_CARE`.
4. **scop:** unchanged — the same spinning cube.

**Verify:**
- `cargo run -p scop -- resources/42.obj`: the cube never shows a back face through a
  front face, at any angle.
- Validation layer silent, including at exit.
- Experiment: set the compare op to `GREATER` → only the background. Put it back.
- `cargo test`, `cargo clippy --all-targets` clean.

---

## Questions

Q-11.1 (recall) What does the depth buffer hold for each pixel, what is it reset to at
the start of every frame, and why that value?

Q-11.2 (prediction) The depth test is on but **depth write is off**. Nothing else
changes. What do you see on the spinning cube, and why?

Q-11.3 (prediction) You set the compare op to `GREATER` and keep clearing to 1.0. What
do you see, and why?

Q-11.4 (application) The colour image's store op is `STORE`, but the depth image's is
`DONT_CARE`. Why is throwing the depth numbers away at the end of the frame fine?

Q-11.5 (prediction) To let the camera get really close to the logo, you change `near`
from 0.1 to 0.0001. What might go wrong, and where on screen would you notice it first?

Answers: `ANSWERS.md` § 11
