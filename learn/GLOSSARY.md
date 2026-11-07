# Glossary — scop

Terms in the order they were introduced. Plain language first, then precise.
Identifiers keep their English names.

---

## Section 1 — Rust

### ownership
**Plain:** every value has exactly one variable responsible for cleaning it up.
**Precise:** when the owner goes out of scope, the value is dropped (its `Drop` runs,
memory is freed). Ownership can be moved but never duplicated for non-`Copy` types.
**Introduced in:** § 1

### move
**Plain:** handing a value to someone else; you can't use it any more afterwards.
**Precise:** assignment or passing by value transfers ownership for non-`Copy` types;
the source binding becomes invalid (`E0382` if used). `Copy` types are bit-copied instead.
**Introduced in:** § 1

### borrow
**Plain:** lending a value without giving it away.
**Precise:** `&T` (shared, read-only, any number) or `&mut T` (exclusive, read-write,
exactly one, no shared borrows alive at the same time). Enforced at compile time by
the borrow checker.
**Introduced in:** § 1

### lifetime
**Plain:** how long a borrow is allowed to live — never longer than the thing it points to.
**Precise:** a compile-time region, usually inferred, written `'a` when annotated.
Prevents dangling references.
**Introduced in:** § 1

### Option
**Plain:** "maybe a value". Rust's replacement for NULL.
**Precise:** `enum Option<T> { Some(T), None }`. You must handle `None` to get at the value.
**Introduced in:** § 1

### Result
**Plain:** "a value or an error". Rust's replacement for return codes and `errno`.
**Precise:** `enum Result<T, E> { Ok(T), Err(E) }`.
**Introduced in:** § 1

### ? operator
**Plain:** "if this failed, return the error from my function now; otherwise give me the value."
**Precise:** early-returns `Err(From::from(e))`; only usable in functions returning
`Result` or `Option`. Error types must match or be convertible via `From`.
**Introduced in:** § 1

### trait
**Plain:** a set of functions a type promises to provide — an interface.
**Precise:** e.g. `Debug`, `Clone`, `Copy`, `Drop`, `From`. Implemented with `impl Trait for Type`.
**Introduced in:** § 1

### Drop
**Plain:** Rust's destructor — runs automatically when a value goes out of scope.
**Precise:** trait with `fn drop(&mut self)`. Struct fields drop in declaration order;
local variables in reverse declaration order.
**Introduced in:** § 1
**You'll see it in:** every Vulkan wrapper (§ 3 onwards)

### RAII
**Stands for:** Resource Acquisition Is Initialisation
**Plain:** tie a resource to a value's lifetime: create the value → get the resource;
drop the value → release it.
**Precise:** a C++ idiom; in Rust it is implemented through `Drop`. The name only
describes the acquiring half — the releasing half is the useful part.
**Introduced in:** § 1

### crate
**Plain:** one Rust library or one executable.
**Precise:** the unit of compilation; a library crate (`lib.rs`) or binary crate (`main.rs`).
**Introduced in:** § 1

### Cargo workspace
**Plain:** several crates in one repo, built together.
**Precise:** a root `Cargo.toml` with `[workspace] members = [...]`; shared `target/`
and `Cargo.lock`. scop: `engine` (lib) + `scop` (bin).
**Introduced in:** § 1

### unsafe
**Plain:** a block where you take responsibility for the checks the compiler can't do — like writing C.
**Precise:** permits dereferencing raw pointers, calling `unsafe fn` (all `ash` Vulkan
calls), mutable statics, `unsafe` traits and union fields. Borrow rules still apply to references.
**Introduced in:** § 1

### raw pointer
**Plain:** a plain C pointer.
**Precise:** `*const T` / `*mut T`: no ownership or lifetime tracking; may be null or
dangling. Creating is safe; dereferencing is `unsafe`.
**Introduced in:** § 1

### CString
**Plain:** an owned, null-terminated string, the shape C APIs expect.
**Precise:** `std::ffi::CString`; `.as_ptr()` gives `*const c_char` valid only while the
`CString` lives.
**Introduced in:** § 1

### repr(C)
**Stands for:** representation C
**Plain:** "lay this struct out exactly like C would".
**Precise:** `#[repr(C)]` fixes field order and padding to the C ABI. Required for data
the GPU reads byte-for-byte (vertices, uniforms).
**Introduced in:** § 1

---

## Section 2 — How a GPU draws

### CPU
**Stands for:** Central Processing Unit
**Plain:** the main processor — a few fast, clever cores. Runs your Rust code.
**Introduced in:** § 2

### GPU
**Stands for:** Graphics Processing Unit
**Plain:** thousands of simple cores that all run the same small program on different data at once.
**Precise:** a separate processor you send work to via an API; asynchronous with the CPU.
**Introduced in:** § 2

### API
**Stands for:** Application Programming Interface
**Plain:** the set of functions a library or system offers you.
**Introduced in:** § 2

### Vulkan
**Plain:** an explicit, low-level graphics API: you configure everything, the driver guesses nothing.
**Precise:** Khronos Group, 2016, successor to AMD's Mantle. Named after Vulcan, god of the forge.
**Introduced in:** § 2

### MoltenVK
**Stands for:** "Molten" + VK (Vulkan)
**Plain:** a layer that runs Vulkan on top of Apple's Metal, because macOS has no native Vulkan.
**Precise:** implements (most of) Vulkan as a "portability" implementation; requires
`VK_KHR_portability_enumeration` at instance creation.
**Introduced in:** § 2

### rendering pipeline
**Plain:** the fixed production line from vertices to pixels.
**Precise:** vertex shader → primitive assembly → rasterisation → fragment shader →
depth test/blending → framebuffer.
**Introduced in:** § 2

### vertex
**Plain:** one corner point, plus any data attached to it (colour, texture coordinates).
**Precise:** plural *vertices*; the per-vertex input to the vertex shader.
**Introduced in:** § 2

### primitive
**Plain:** the smallest shape the GPU draws — for you, a triangle.
**Introduced in:** § 2

### vertex shader
**Plain:** your GPU program that runs once per vertex and decides where it lands on screen.
**Precise:** outputs `gl_Position` in clip space, plus any values to interpolate.
**Introduced in:** § 2

### rasterisation
**Plain:** working out which pixels a triangle covers.
**Precise:** fixed-function stage producing fragments and interpolating vertex outputs
across the triangle. From *raster*, the scan-line grid of old screens.
**Introduced in:** § 2

### fragment
**Plain:** a candidate pixel — "this triangle covers this pixel" — before it's known to be visible.
**Introduced in:** § 2

### fragment shader
**Plain:** your GPU program that runs once per fragment and decides its colour.
**Precise:** DirectX calls it a pixel shader.
**Introduced in:** § 2

### framebuffer
**Plain:** the image(s) being drawn into — colour, and later depth.
**Introduced in:** § 2

### swapchain
**Plain:** 2–3 images that take turns: one on screen, one being drawn.
**Precise:** `VkSwapchainKHR`, created from a surface (§ 4).
**Introduced in:** § 2

### NDC
**Stands for:** Normalised Device Coordinates
**Plain:** a fixed box the vertex shader's output ends up in: −1..+1 across the window.
**Precise:** clip space divided by `w`. In Vulkan: x right, **y down**, depth 0..1
(OpenGL: y up, depth −1..1).
**Introduced in:** § 2

### GLSL
**Stands for:** OpenGL Shading Language
**Plain:** the C-like language you write shaders in. Works for Vulkan despite the name.
**Introduced in:** § 2

### SPIR-V
**Stands for:** Standard Portable Intermediate Representation, version 5
**Plain:** compiled shader bytes — Vulkan only accepts this, never GLSL text.
**Precise:** produced by `glslc`; loaded by you into a `VkShaderModule` (§ 6).
**Introduced in:** § 2

### command buffer
**Plain:** a recorded shopping list of GPU work, handed over in one go.
**Precise:** `VkCommandBuffer`, recorded on the CPU, submitted to a queue (§ 5).
**Introduced in:** § 2

### queue
**Plain:** the GPU's inbox for command buffers. Submitting returns immediately.
**Precise:** `VkQueue`; work completes asynchronously, signalled by fences/semaphores.
**Introduced in:** § 2

### winit
**Stands for:** window initialisation
**Plain:** the Rust crate that makes the window and delivers key/mouse/resize events — your mlx.
**Precise:** since 0.30, implement `ApplicationHandler`; create windows in `resumed`.
**Introduced in:** § 2

### event loop
**Plain:** the loop that waits for things to happen and calls your code for each — like `mlx_loop` with hooks.
**Introduced in:** § 2

---

## Section 3 — Talking to the GPU

### Vulkan loader
**Plain:** the switchboard library that finds your GPU's driver and forwards calls to it.
**Precise:** `libvulkan.dylib` (macOS) / `libvulkan.so` (Linux); loaded by `ash::Entry::load()`.
**Introduced in:** § 3

### ICD
**Stands for:** Installable Client Driver
**Plain:** a Vulkan driver the loader can find via a JSON file — MoltenVK is yours.
**Introduced in:** § 3

### VkInstance
**Plain:** your program's connection to Vulkan; created first, destroyed last.
**Introduced in:** § 3

### extension
**Plain:** an optional Vulkan feature you must ask for by name.
**Precise:** instance extensions (at instance creation) or device extensions (at device
creation). `KHR` = official Khronos, `EXT` = multi-vendor.
**Introduced in:** § 3

### layer
**Plain:** code inserted between your program and the driver that sees every call.
**Introduced in:** § 3

### validation layer
**Plain:** the inspector that checks every Vulkan call against the rules and reports mistakes.
**Precise:** `VK_LAYER_KHRONOS_validation`; enable in debug builds only.
**Introduced in:** § 3

### debug messenger
**Plain:** the object that passes validation messages to a function you wrote, so you can print them.
**Precise:** `VkDebugUtilsMessengerEXT`, from `VK_EXT_debug_utils`.
**Introduced in:** § 3

### VK_KHR_portability_enumeration
**Stands for:** Khronos — portability — enumeration (listing)
**Plain:** the opt-in that makes the loader list "almost-complete" drivers like MoltenVK.
**Precise:** instance extension; also requires the `ENUMERATE_PORTABILITY_KHR` instance flag.
**Introduced in:** § 3

### VK_KHR_portability_subset
**Plain:** the device-side acknowledgement that the driver implements a subset of Vulkan.
**Precise:** device extension; must be enabled if the device lists it.
**Introduced in:** § 3

### physical device
**Plain:** one real GPU on the machine; you list them and choose one.
**Precise:** `VkPhysicalDevice`; not created or destroyed by you.
**Introduced in:** § 3

### queue family
**Plain:** a group of GPU inboxes with the same abilities (draw, compute, copy), identified by an index.
**Introduced in:** § 3

### logical device
**Plain:** your session with the chosen GPU; nearly everything is created from it.
**Precise:** `VkDevice`; destroy everything made from it first.
**Introduced in:** § 3

### handle
**Plain:** an opaque number naming a Vulkan object — like a file descriptor. Copying it doesn't copy the object.
**Introduced in:** § 3

### CreateInfo
**Plain:** the struct you fill in to describe an object before creating it.
**Precise:** `Vk*CreateInfo`, starting with `sType` (structure type) and `pNext` (extension chain).
**Introduced in:** § 3

### VkResult
**Plain:** the success/error code almost every Vulkan function returns.
**Precise:** in `ash`, surfaced as `Result<T, vk::Result>`.
**Introduced in:** § 3

### ash
**Plain:** the Rust crate that exposes Vulkan's C functions 1:1. A pun: what's left after Vulkan's fire.
**Precise:** `ash::Entry` (loader), `ash::Instance` / `ash::Device` (function tables); calls are `unsafe`.
**Introduced in:** § 3

---

## Section 4 — Getting images to the screen

### surface
**Plain:** Vulkan's adaptor to your window — "somewhere I can present images".
**Precise:** `VkSurfaceKHR`; created from the instance and the window; the window must outlive it.
**Introduced in:** § 4

### VK_KHR_surface
**Plain:** the instance extension that defines surfaces in general.
**Introduced in:** § 4

### VK_EXT_metal_surface
**Plain:** the macOS-specific instance extension that makes a surface from a window's Metal layer.
**Introduced in:** § 4

### raw-window-handle
**Plain:** a tiny crate defining a common "here's a window handle" format between window and graphics libraries.
**Introduced in:** § 4

### ash-window
**Plain:** a small crate that turns a raw window handle into the right Vulkan surface and lists the extensions needed.
**Introduced in:** § 4

### present
**Plain:** hand a finished image over to be shown on screen.
**Introduced in:** § 4

### VK_KHR_swapchain
**Plain:** the device extension that provides swapchains.
**Introduced in:** § 4

### surface capabilities
**Plain:** what the surface allows: image count limits, current size, size limits, transforms, alpha modes.
**Introduced in:** § 4

### surface format
**Plain:** the pixel layout and colour space of the swapchain images, e.g. `B8G8R8A8_SRGB`.
**Introduced in:** § 4

### sRGB
**Stands for:** standard Red Green Blue
**Plain:** a colour encoding that spends more precision on dark shades; `_SRGB` formats convert automatically on write.
**Introduced in:** § 4

### present mode
**Plain:** the rule for when a finished image replaces the one on screen.
**Introduced in:** § 4

### FIFO
**Stands for:** First In, First Out
**Plain:** vsync: one queued image shown per screen refresh. Always supported.
**Introduced in:** § 4

### MAILBOX
**Plain:** like FIFO, but a newer finished image replaces the waiting one. Optional.
**Introduced in:** § 4

### extent
**Plain:** a 2D size in pixels (`width × height`). Always physical pixels.
**Introduced in:** § 4

### VkImage
**Plain:** a block of GPU memory holding pixels, plus its description. Swapchain images are owned by the swapchain.
**Introduced in:** § 4

### image view
**Plain:** a description of how to read an image (format, which part); rendering always goes through a view.
**Precise:** `VkImageView`; you create and destroy one per swapchain image.
**Introduced in:** § 4

### ERROR_OUT_OF_DATE_KHR
**Plain:** "the swapchain no longer matches the window" — rebuild it. Sibling: `SUBOPTIMAL_KHR`.
**Introduced in:** § 4

### composite alpha
**Plain:** how the OS should treat your image's transparency when it layers your window over the rest of the screen. `OPAQUE` = ignore it, the window is solid.
**Precise:** `VkCompositeAlphaFlagBitsKHR`, chosen at swapchain creation from `supported_composite_alpha` in the surface capabilities.
**Introduced in:** § 4

---

## Section 5 — Giving the GPU work, and waiting

### command pool
**Plain:** the memory command buffers are carved from; one per queue family.
**Precise:** `VkCommandPool`; `RESET_COMMAND_BUFFER` lets you reset buffers one at a time. Destroying the pool frees its buffers.
**Introduced in:** § 5

### recording
**Plain:** writing instructions into a command buffer between `begin` and `end`. Nothing runs on the GPU yet.
**Precise:** every `cmd_*` call (`cmd_clear_color_image`, `cmd_pipeline_barrier`, …).
**Introduced in:** § 5

### vkQueueSubmit
**Plain:** hand command buffers to a queue; returns at once, the GPU runs them later.
**Precise:** also names semaphores to wait on (and at which stage), semaphores to signal, and one fence to signal when done.
**Introduced in:** § 5

### fence
**Plain:** a signal from the GPU to the CPU: "that batch of work is finished". **GPU signals, CPU waits.**
**Precise:** `VkFence`; `wait_for_fences` / `reset_fences`. Create frame fences `SIGNALED` or the first frame waits forever.
**Introduced in:** § 5

### semaphore
**Plain:** a signal between GPU jobs (or the GPU and the screen): "you may go now". **GPU signals, GPU/presentation waits** — the CPU never waits on it. Named after railway semaphore signals.
**Precise:** `VkSemaphore` (binary). scop uses `image_free` (per frame in flight) and `render_done` (per swapchain image).
**Introduced in:** § 5

### acquire
**Plain:** ask the swapchain which image to draw into next; you get the index at once, and a semaphore is raised when the image is really off screen.
**Precise:** `vkAcquireNextImageKHR`; may return `ERROR_OUT_OF_DATE_KHR` or report suboptimal.
**Introduced in:** § 5

### frames in flight
**Plain:** how many frames the CPU may work ahead of the GPU. 2 lets the CPU record the next frame while the GPU draws the current one.
**Precise:** each frame slot has its own command buffer, fence and `image_free` semaphore.
**Introduced in:** § 5

### image layout
**Plain:** how an image's pixels are arranged in GPU memory for a particular job (being written, being drawn into, being shown).
**Precise:** `UNDEFINED`, `TRANSFER_DST_OPTIMAL`, `COLOR_ATTACHMENT_OPTIMAL`, `PRESENT_SRC_KHR`, …
**Introduced in:** § 5

### layout transition
**Plain:** switching an image from one layout to another, as a recorded GPU command.
**Precise:** done with an image memory barrier inside `cmd_pipeline_barrier`.
**Introduced in:** § 5

### pipeline barrier
**Plain:** "everything of this kind before here must finish before anything of that kind after here starts" — plus, optionally, a layout change.
**Precise:** `vkCmdPipelineBarrier` with source/destination stages and access masks.
**Introduced in:** § 5

### pipeline stage
**Plain:** one step on the GPU's production line; barriers and waits name stages so only the work that must wait does.
**Precise:** `TRANSFER`, `COLOR_ATTACHMENT_OUTPUT`, `TOP_OF_PIPE`, `BOTTOM_OF_PIPE`, …
**Introduced in:** § 5

### access mask
**Plain:** which kind of memory access must be finished and made visible. Stage = *when*, access = *what*.
**Precise:** `TRANSFER_WRITE`, `COLOR_ATTACHMENT_WRITE`, `SHADER_READ`, …; tells the GPU which caches to flush.
**Introduced in:** § 5

### vkCmdClearColorImage
**Plain:** fill a whole image with one colour; no shaders needed.
**Precise:** a transfer command; the image must be in `TRANSFER_DST_OPTIMAL` and have `TRANSFER_DST` usage.
**Introduced in:** § 5

### TRANSFER_DST
**Plain:** "transfer destination" — an image usage flag allowing copies and clears into it.
**Precise:** `vk::ImageUsageFlags::TRANSFER_DST`; must be requested at swapchain creation and supported by the surface.
**Introduced in:** § 5

### device_wait_idle
**Plain:** wait until the GPU has finished everything. For shutdown, never for every frame.
**Precise:** `vkDeviceWaitIdle`.
**Introduced in:** § 5

### RedrawRequested
**Plain:** the `winit` event "draw this window now"; calling `window.request_redraw()` after each frame makes a continuous loop.
**Precise:** `WindowEvent::RedrawRequested`.
**Introduced in:** § 5

---

## Section 6 — Programs that run on the GPU

### shader module
**Plain:** Vulkan's handle for one block of compiled shader code.
**Precise:** `VkShaderModule`, created from SPIR-V words; can be destroyed once the pipeline is built.
**Introduced in:** § 6

### entry point
**Plain:** the function a shader starts in — like `main` in C.
**Precise:** named when building the pipeline (`c"main"`).
**Introduced in:** § 6

### build script
**Plain:** `build.rs` — a small Rust program Cargo runs before compiling the crate. Ours compiles the shaders.
**Precise:** prints `cargo:` lines to talk to Cargo, e.g. `cargo:rerun-if-changed=shaders`; writes outputs to `OUT_DIR`.
**Introduced in:** § 6

### glslc
**Stands for:** GLSL compiler (Google's `shaderc` project).
**Plain:** turns GLSL text into SPIR-V bytes. A build tool, not a library linked into scop.
**Introduced in:** § 6

### include_bytes!
**Plain:** a Rust macro that copies a file's bytes into the program at compile time.
**Precise:** gives `&'static [u8]`; with `concat!(env!("OUT_DIR"), "/x.spv")` it embeds build-script output.
**Introduced in:** § 6

### SPIR-V magic number
**Plain:** the first 4 bytes every SPIR-V file starts with, `0x07230203` — a quick "is this really SPIR-V?" check.
**Precise:** SPIR-V is a list of 32-bit little-endian words, so the size must be a multiple of 4.
**Introduced in:** § 6

### graphics pipeline
**Plain:** the whole drawing production line — your shaders plus every fixed switch — built once and frozen.
**Precise:** `VkPipeline` from `create_graphics_pipelines`. Changing a non-dynamic setting means a new pipeline.
**Introduced in:** § 6

### fixed-function state
**Plain:** the switches for the pipeline stages you don't program (assembly, rasterisation, blending).
**Introduced in:** § 6

### input assembly
**Plain:** how the stream of corners is grouped into shapes.
**Precise:** topology `TRIANGLE_LIST` = every 3 vertices form one triangle.
**Introduced in:** § 6

### viewport
**Plain:** the rectangle of the image that NDC's −1..+1 box is stretched onto.
**Precise:** normally the whole swapchain extent; dynamic state in scop.
**Introduced in:** § 6

### scissor
**Plain:** a rectangle outside of which pixels are thrown away. Viewport squashes, scissor cuts.
**Introduced in:** § 6

### dynamic state
**Plain:** pipeline settings left out of the frozen pipeline and set while recording instead.
**Precise:** scop: `VIEWPORT`, `SCISSOR` (so resizing needs no new pipeline).
**Introduced in:** § 6

### cull mode
**Plain:** whether to skip triangles facing away from the camera.
**Precise:** `NONE`, `BACK`, `FRONT`; which side is "front" is set by front face.
**Introduced in:** § 6

### front face
**Plain:** which corner order (clockwise or counter-clockwise on screen) counts as facing you.
**Introduced in:** § 6

### colour write mask
**Plain:** which colour channels (R, G, B, A) the pipeline may write. Defaults to none — forget it and nothing appears.
**Introduced in:** § 6

### pipeline layout
**Plain:** the list of extra inputs (uniforms, textures) shaders may read. Empty for now, but required.
**Precise:** `VkPipelineLayout`.
**Introduced in:** § 6

### dynamic rendering
**Plain:** starting a drawing pass by naming the image views to draw into, directly in the command buffer.
**Precise:** `cmd_begin_rendering` / `cmd_end_rendering`; replaces render-pass and framebuffer objects.
**Introduced in:** § 6

### VK_KHR_dynamic_rendering
**Plain:** the device extension giving dynamic rendering on Vulkan 1.2 (built in from 1.3).
**Precise:** needs the extension **and** the `dynamicRendering` feature enabled at device creation; functions come from `ash::khr::dynamic_rendering::Device`.
**Introduced in:** § 6

### load op / store op
**Plain:** what happens to the image at the start (clear / keep / don't care) and end (keep / don't care) of drawing.
**Precise:** `AttachmentLoadOp::CLEAR`, `AttachmentStoreOp::STORE` in scop.
**Introduced in:** § 6

### gl_VertexIndex
**Plain:** built-in vertex shader input: which corner this run is for (0, 1, 2…).
**Introduced in:** § 6

### gl_Position
**Plain:** built-in vertex shader output: where the corner lands, as (x, y, z, w).
**Introduced in:** § 6

### location
**Plain:** a numbered slot linking a vertex shader output to a fragment shader input. Numbers must match; names don't matter.
**Precise:** `layout(location = N)`.
**Introduced in:** § 6

### interpolation
**Plain:** blending per-corner values across a triangle for each pixel, weighted by distance to each corner. Done by the rasteriser.
**Introduced in:** § 6
