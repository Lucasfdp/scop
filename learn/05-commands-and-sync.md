---
section: 5
slug: commands-and-sync
tier: T2
status: in-progress
prereqs: [4]
terms: [command pool, recording, vkQueueSubmit, fence, semaphore, acquire, frames in flight, image layout, layout transition, pipeline barrier, pipeline stage, access mask, vkCmdClearColorImage, TRANSFER_DST, device_wait_idle, RedrawRequested]
---

# Section 5 — Giving the GPU work, and waiting

**Depth: T2** — every frame scop ever draws goes through the loop you build here, and
almost every "it works on my Mac but flickers / freezes / crashes on campus" bug is a
synchronisation bug. You need to be able to explain *why* each wait is where it is.
**You'll be able to:** record GPU work, send it, show the result on screen every frame,
and keep the CPU, the GPU and the screen from treading on each other.

---

## The idea

Back to § 4's theatre. You — the program on the CPU — are now the **director**. You
don't paint; you write instruction sheets ("paint this frame solid blue") and hand them
to the painters (the GPU). Painters work on their own, at their own speed. While they
paint, you're free to do something else — like writing the next sheet.

That freedom is the whole point, and it's also the danger. Three things are going on at
once, and none of them waits for the others unless you say so:

1. **You, writing sheets.** If you rewrite a sheet while the painters are still reading
   it, they paint nonsense. So before reusing a sheet, you need to know the painters are
   finished with it. That's a **light on your desk** that the painters switch on when
   they're done with a job. You can stop and wait for the light.
2. **The painters.** They mustn't start painting a frame that's still on stage. So the
   stage manager raises a **flag** when a frame comes back free — and the painters wait
   for the flag before starting. You never see this flag; it's between the stage
   manager and the painters.
3. **The stage manager.** They mustn't put a frame on stage while it's still wet. So
   the painters raise a second **flag** when they finish, and the stage manager waits
   for it.

Notice the difference between the light and the flags. The **light** is for *you* —
the CPU stops and waits on it. The **flags** are for the painters and the stage manager
— they coordinate among themselves, and you just tell them which flags to use.

One more problem. Before you paint on a canvas you prepare it one way (primed,
stretched flat); before it goes on stage you prepare it another way (framed, varnished).
The canvas is the same — only how it's laid out for the job changes. GPUs are the same:
the same image is arranged differently in memory depending on whether it's about to be
painted, read, or displayed. You must say when to switch, and switching is itself an
instruction on the sheet.

**Where the analogy stops being true:** painters don't read sheets in a sensible order
just because you wrote them in order — the GPU may overlap steps unless told not to.
And "the light" and "the flags" are just small objects you create and reuse every frame;
nothing physical changes.

---

## The names for it

**command pool** — `VkCommandPool`: the memory that command buffers are carved from.
One pool per queue family (command buffers can only be sent to queues of the family
their pool was made for). You create the pool; you *allocate* command buffers from it.
The flag `RESET_COMMAND_BUFFER` lets you reset one buffer at a time instead of the whole
pool.

**command buffer** — already in the glossary from § 2: the instruction sheet.
New here is its life cycle: *allocate* (once) → *begin* → record commands → *end* →
*submit* → wait until the GPU is done → *reset* → begin again.

**recording** — calling `cmd_*` functions on a command buffer between `begin` and
`end`. Recording does **nothing** on the GPU: it only writes the sheet. Every
recording function starts with `cmd` — `cmd_clear_color_image`, `cmd_pipeline_barrier`.

**`vkQueueSubmit`** — "**V**ul**k**an: **Queue Submit**". Hands command buffers to a
queue. It returns immediately; the GPU runs the work later. The submit also says which
flags (semaphores) to wait for before starting, which to raise when finished, and which
light (fence) to switch on when finished.

**fence** — `VkFence`: the light on your desk. **GPU signals, CPU waits.**
`wait_for_fences` blocks the CPU until it's on; `reset_fences` switches it off again.
It's called a fence because work on one side of it can't be confused with work on the
other.

**semaphore** — `VkSemaphore`: a flag. **GPU signals, GPU (or the presentation engine)
waits.** The CPU never waits on these semaphores. The name comes from railway
*semaphore signals* — the arms on a post that tell a train whether it may proceed onto
the next stretch of track. (Strictly these are *binary* semaphores: up or down. Vulkan
also has counting "timeline" semaphores; scop doesn't need them.)

**acquire** — `vkAcquireNextImageKHR`: "give me the index of a swapchain image I may
draw into next". It returns at once with an index — but the image may still be on
stage. The semaphore you pass is raised when it really is free.

**frames in flight** — how many frames the CPU may be ahead of the GPU. With **1**,
you write a sheet, wait for the painters to finish it, then write the next: CPU and GPU
take turns, each idle half the time. With **2**, you write frame 2's sheet while the GPU
paints frame 1. Each frame in flight needs its own command buffer, fence and
"image free" semaphore. Two is the usual choice: more adds input delay for little gain.

**image layout** — how an image's pixels are arranged in GPU memory for a particular
job. `UNDEFINED` (contents don't matter — may be thrown away), `TRANSFER_DST_OPTIMAL`
(best for being *written to* by a copy or clear), `COLOR_ATTACHMENT_OPTIMAL` (best for
drawing into, from § 6), `PRESENT_SRC_KHR` (ready to be shown: *present source*). The
GPU may compress or tile pixels differently for each.

**layout transition** — switching an image from one layout to another. Recorded on the
command buffer, done by the GPU.

**pipeline barrier** — `vkCmdPipelineBarrier`: a recorded command that says "everything
of *kind X* before this point must be finished before anything of *kind Y* after it
starts — and while you're at it, change this image's layout". It's the "don't overlap
these" instruction from the analogy. *Pipeline* because the GPU works like a production
line with stages; a *barrier* stops some of them.

**pipeline stage** — one step on that production line: `TRANSFER` (copies and clears),
`COLOR_ATTACHMENT_OUTPUT` (writing drawn pixels, § 6), `TOP_OF_PIPE` / `BOTTOM_OF_PIPE`
("before anything" / "after everything"). Barriers and semaphore waits name stages so
the GPU knows exactly *what* has to wait, and can keep everything else running.

**access mask** — *which kind of memory access* must be finished or made visible:
`TRANSFER_WRITE`, `COLOR_ATTACHMENT_WRITE`, `SHADER_READ`… A stage says *when*, an
access mask says *what*. GPUs keep caches; the access mask is how the barrier knows
which writes must be flushed so the next step sees them.

**`vkCmdClearColorImage`** — **V**ul**k**an **C**o**m**man**d**: clear a colour image to
one colour. The simplest thing that changes pixels, with no shaders or pipeline
involved. It's a transfer operation, so the image must be in `TRANSFER_DST_OPTIMAL`.

**`TRANSFER_DST`** — **transfer destination**: an image *usage* flag meaning "may be the
target of copies and clears". Swapchain images only get the usages you ask for when
creating the swapchain — so § 4's `COLOR_ATTACHMENT` alone isn't enough to clear them.

**`device_wait_idle`** — "wait until the device has finished *everything*". Heavy-handed
— never in the frame loop — but exactly right before destroying things at exit.

**`RedrawRequested`** — the `winit` event meaning "now is a good time to draw this
window". You get one when the OS asks, or when you call `window.request_redraw()`.
Calling it at the end of every frame gives you a continuous loop.

---

## How it actually works

### Frames, frame slots and images — three different things

These get mixed up easily, so pin them down first:

- **A frame** is one trip through the loop: one picture drawn and shown. It happens
  ~60 times a second, forever.
- **A frame slot** (2 of them — "frames in flight") is *your* reusable kit: one
  instruction sheet (command buffer), one desk light (fence), one "image free" flag
  (semaphore). You pick slots **in turn**: 0, 1, 0, 1… Two slots means the CPU can be
  at most two frames ahead of the GPU.
- **A swapchain image** (3 on your Mac) is a *canvas*: actual pixels the screen shows.
  You don't pick it — **acquire tells you** which one is free, in whatever order the
  presentation engine hands them back. Each image has its own view (§ 4) and its own
  "render done" flag.

| Loop | Frame slot (you pick, in turn) | Image (acquire picks) |
|---|---|---|
| 1 | slot 0 | image 0 |
| 2 | slot 1 | image 1 |
| 3 | slot 0 | image 2 |
| 4 | slot 1 | image 0 |
| 5 | slot 0 | image 1 |

The two counts are independent: **3 images** is how many canvases take turns on screen
(the swapchain decided, § 4); **2 slots** is how far ahead you let the CPU run (your
choice). Rule of thumb for *where an object belongs*: if the **CPU reuses it**, it's per
slot; if the **screen holds on to it**, it's per image.

### The whole frame, at a glance

```
CPU (your code)                           GPU / presentation engine
───────────────                           ─────────────────────────
wait_for_fences(frame.fence)   ◄────────  fence signalled when frame N-2 finished
acquire_next_image(..., frame.image_free) ─► image_free raised when the image is off stage
reset_fences(frame.fence)
reset + record command buffer:
   barrier: UNDEFINED → TRANSFER_DST_OPTIMAL
   clear to colour
   barrier: TRANSFER_DST_OPTIMAL → PRESENT_SRC_KHR
queue_submit:
   wait  image_free  (at TRANSFER stage) ──►  GPU waits, then runs the commands
   signal render_done[image]             ──►  raised when the commands are finished
   fence frame.fence                     ──►  switched on when the commands are finished
queue_present:
   wait render_done[image]               ──►  stage manager shows the image
frame = (frame + 1) % 2
```

Everything on the left returns quickly. The only place the CPU *stops* is
`wait_for_fences` (and, sometimes, `acquire`).

### 1. Creating the objects (once)

- **Command pool** for the graphics queue family, with `RESET_COMMAND_BUFFER`.
- **Per frame in flight (2):** one command buffer, one fence, one `image_free`
  semaphore.
- **Per swapchain image (3 on your Mac):** one `render_done` semaphore. Why per *image*
  and not per frame: the presentation engine holds on to `render_done` until it actually
  shows that image, which can be later than your next use of the same frame slot. Tying
  it to the image means you only reuse it once that image comes back.
- **Fences are created already signalled** (`FenceCreateFlags::SIGNALED`). The first
  frame starts by waiting on its fence; if the light starts off, you wait forever for a
  job that was never sent.

Swapchain creation (§ 4) also needs `TRANSFER_DST` added to its image usage — check the
surface's `supported_usage_flags` includes it.

### 2. Recording

```
begin_command_buffer(cmd, ONE_TIME_SUBMIT)       // "this recording is used once"
pipeline_barrier(
    src stage TRANSFER, src access none,         // wait for the semaphore's stage
    dst stage TRANSFER, dst access TRANSFER_WRITE,
    image: UNDEFINED → TRANSFER_DST_OPTIMAL)
clear_color_image(image, TRANSFER_DST_OPTIMAL, colour, whole image)
pipeline_barrier(
    src stage TRANSFER, src access TRANSFER_WRITE, // the clear must finish…
    dst stage BOTTOM_OF_PIPE, dst access none,     // …before the frame counts as done
    image: TRANSFER_DST_OPTIMAL → PRESENT_SRC_KHR)
end_command_buffer(cmd)
```

`UNDEFINED` as the old layout means "I don't care what was there" — the GPU may skip
preserving it. Fine here: you overwrite every pixel. The "whole image" is a
*subresource range*: colour aspect, mip level 0, one level, layer 0, one layer — the
same range as § 4's image views.

### 3. Submitting

The submit waits on `image_free` **at the `TRANSFER` stage**. That's precise: the GPU
may start any work that comes *before* transfer straight away, and only holds back from
the clear. This is the "stage says when" idea from the names section — and the first
barrier's source stage is `TRANSFER` so it lines up with this wait.

### 4. Presenting

`queue_present` with the swapchain, the image index, and `render_done[image]` to wait
on. Like submit, it returns at once.

### 5. Acquire and present can say "the window changed"

Both can return `ERROR_OUT_OF_DATE_KHR` ("the swapchain no longer matches the window —
rebuild it") or report *suboptimal* ("still works, but no longer a perfect match").
Rebuilding is § 16. For now: on out-of-date, skip the frame. **Important:** if acquire
fails, return *before* `reset_fences` — otherwise the fence is off with no job to
switch it on, and the next frame using that slot waits forever.

### 6. Driving it from `winit`

- On `RedrawRequested`: if the window is minimised (0×0), do nothing. Otherwise draw a
  frame, then call `window.request_redraw()` to ask for the next one.
- With `FIFO`, presenting blocks at roughly your screen's refresh rate, so this loop
  doesn't run flat out.

### 7. Shutting down

Before destroying anything: `device_wait_idle()`. Then destroy in the mirror order —
semaphores and fences, then the command pool (which frees its command buffers), then
§ 4's objects.

### 8. Ownership in Rust

The frame objects go in the engine, in their own struct owned by `GpuContext`, destroyed
in its `Drop` *after* `device_wait_idle` and *before* the swapchain. The `render_done`
semaphores belong with the swapchain images (one per image), because if the swapchain is
rebuilt (§ 16) with a different image count, they must be rebuilt too.

### What about render passes vs dynamic rendering?

You've seen this open thread in `PROGRESS.md`. Clearing with `vkCmdClearColorImage`
needs neither, so the decision moves to § 6, where the graphics pipeline does need one
— the trade-offs will be laid out there. MoltenVK on your Mac reports **Vulkan 1.2**,
which is the main input.

---

## Known traps

(The `VUID-…` numbers below are what recent validation layers print; an older layer may
word them slightly differently.)

- **Hangs on the very first frame** — the fence was created unsignalled. Create it with
  `SIGNALED`.
- **Hangs after resizing** — you reset the fence and then returned early because acquire
  said out-of-date. Only reset after acquire succeeds.
- **`VUID-vkBeginCommandBuffer-commandBuffer-00049`** ("command buffer is in use") — you
  re-recorded a command buffer the GPU is still running. You skipped or misplaced the
  fence wait.
- **`VUID-vkQueueSubmit-pSignalSemaphores-00067`** or a warning about a semaphore still
  in use by present — `render_done` was per frame instead of per swapchain image.
- **`VUID-VkPresentInfoKHR-pImageIndices-01430`** ("image layout … expected
  `VK_IMAGE_LAYOUT_PRESENT_SRC_KHR`") — you forgot the final layout transition.
- **Validation error that the image wasn't created with `TRANSFER_DST`** — you didn't
  add the usage flag to the swapchain.
- **Errors at exit about objects "in use"** — you destroyed things while the GPU was
  still busy. Call `device_wait_idle` first.
- **"I cleared to 0.5 grey and it looks too light"** — the swapchain format is `_SRGB`
  (§ 4). Your 0.5 means "half the light", which the sRGB encoding stores as about 0.73.
  Correct behaviour, not a bug.
- **Only one frame ever drawn** — you didn't call `request_redraw()` after drawing. A
  plain colour hides this; a colour that changes over time reveals it.

---

## Checkpoint

**Satisfies:** "inside a window" — the window now shows pixels you put there, every
frame. (Method: gate first, then Claude writes it and walks you through it.)

1. Swapchain images get `TRANSFER_DST` usage (checked against the surface).
2. An engine struct for per-frame objects: command pool, 2 command buffers, 2 fences
   (signalled), 2 `image_free` semaphores; `render_done` semaphores, one per swapchain
   image.
3. `draw_frame(clear_colour)` doing exactly the loop above, including the out-of-date
   handling.
4. `winit` drives it from `RedrawRequested`, requests the next redraw, and skips while
   minimised.
5. The colour pulses slowly over time — proof that every frame is drawn.
6. `device_wait_idle` before teardown; clean destruction.
7. **Prove the fence matters:** create the fences without `SIGNALED`, run, see it hang,
   put it back.

**Verify:** a slowly pulsing window; no validation messages while running or on exit;
`cargo clippy` clean. Resizing may stop the pulsing (out-of-date) — expected until § 16.

---

## Questions

Q-5.1 (recall) For a fence and for a semaphore: who signals it, and who waits on it?

Q-5.2 (prediction) You create the two frame fences without the `SIGNALED` flag. What
happens when you run scop, and why?

Q-5.3 (application) Put these in order for one frame, and say which semaphore each
submit/present step waits on or raises: *submit*, *present*, *wait for fence*,
*acquire*, *record*, *reset fence*.

Q-5.4 (prediction) You delete the second barrier (the one that moves the image to
`PRESENT_SRC_KHR`). What does validation report, and what might you see on a driver
that doesn't complain?

Q-5.5 (prediction) You switch to 1 frame in flight: one command buffer, one fence. The
program still works. What is the CPU doing while the GPU draws, and what does the second
set of objects let it do instead?

Answers: `ANSWERS.md` § 5
