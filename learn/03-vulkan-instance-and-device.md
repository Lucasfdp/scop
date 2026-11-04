---
section: 3
slug: vulkan-instance-and-device
tier: T2
status: not-started
prereqs: [2]
terms: [Vulkan loader, ICD, VkInstance, extension, layer, validation layer, debug messenger, VK_KHR_portability_enumeration, VK_KHR_portability_subset, physical device, queue family, logical device, handle, CreateInfo, VkResult, ash]
---

# Section 3 — Talking to the GPU

**Depth: T2** — every later Vulkan object is created *from* the device you make
here, and every Vulkan bug you will ever have is reported by the validation layer
you switch on here. Get this right once and it's reusable in every future project
almost unchanged.
**You'll be able to:** start Vulkan on your Mac, pick the GPU, open a session with
it, get a queue to send work to, and see Vulkan's own error reports in your terminal.

---

## The idea

Before you can draw anything, you need three introductions, a bit like phoning a
company:

1. **Get through the switchboard.** Your program calls a small library installed on
   the system. It doesn't draw anything itself; its only job is to find the real
   driver for your GPU and pass your calls along. You tell it who you are and which
   optional features you'd like.
2. **Choose who to talk to.** The switchboard lists the GPUs on the machine. A gaming
   PC might have two (one built into the processor, one graphics card). You look at
   what each can do and choose one. On your Mac there is exactly one.
3. **Open a line with that GPU.** You open a session with the chosen GPU, say which
   extra features you need, and receive an "inbox" — the place you'll post your
   shopping lists of work (§ 2's command buffers).

Vulkan also gives you an optional **inspector**: a checking layer that sits between
your program and the driver and reads every call you make. If you pass a wrong
value, forget to destroy something, or break a rule from the specification, it
prints a detailed message. Without it, a mistake often does *nothing visible* — or
crashes three calls later somewhere unrelated. **You will develop with it switched
on, always.**

**Where the phone analogy stops being true:** a phone call has one line. In Vulkan
the GPU offers several *kinds* of inbox (for drawing, for pure computation, for
copying data), and you must ask for one that can draw.

---

## The names for it

**Vulkan loader** — the switchboard: `libvulkan.dylib` on macOS (`libvulkan.so` on
Linux). It finds the installed drivers and forwards your calls. Homebrew's
`vulkan-loader` installed it.

**ICD** — **I**nstallable **C**lient **D**river. A Vulkan driver the loader finds
through a small JSON file (`MoltenVK_icd.json` on your Mac). "Installable" because
several can be installed side by side; "client" because it runs inside your
program's process, as opposed to the kernel part of the driver.

**`VkInstance`** — your connection to the loader: step 1. `Vk` is the prefix on every
Vulkan type (`vk` on functions, `VK_` on constants). Created first, destroyed last.

**extension** — an optional feature that isn't in core Vulkan. There are **instance
extensions** (asked for when creating the instance, e.g. "I'll need to draw into a
window") and **device extensions** (asked for when creating the logical device,
e.g. "I want a swapchain"). Names follow `VK_<who>_<what>`: `KHR` = Khronos, the
official ones; `EXT` = multi-vendor; vendor tags like `NV`, `AMD` for one vendor.

**layer** — code the loader inserts *between* your program and the driver. Every call
passes through it. Used for checking, logging and debugging.

**validation layer** — the inspector: `VK_LAYER_KHRONOS_validation`. Checks every call
against the specification and reports violations. Homebrew's
`vulkan-validationlayers` installed it. It costs performance, so it's for development
builds only.

**debug messenger** — the object that receives the validation layer's messages and
calls a function you provide (so you can print them). Comes from the instance
extension `VK_EXT_debug_utils` ("debug utilities").

**`VK_KHR_portability_enumeration`** — **enumerate** (list) **portability** drivers.
MoltenVK is a *portability* implementation: it implements almost all of Vulkan, but
not 100%, because Metal underneath can't do everything. The loader hides such drivers
unless you enable this instance extension **and** set the flag
`ENUMERATE_PORTABILITY_KHR` on the instance. Forget either and Vulkan reports no GPU.
Most tutorials (written for Windows/Linux) don't mention it.

**`VK_KHR_portability_subset`** — the device-side partner: "I know this driver only
implements a subset". If a device lists it as available, you are **required** to
enable it when creating the logical device.

**physical device** — `VkPhysicalDevice`: one real GPU, as reported by the instance.
You don't create it; you list them and choose. Read-only information: name, type,
limits, supported features.

**queue family** — a group of queues on a GPU with the same capabilities, identified
by an index (0, 1, 2…). Each family has flags: `GRAPHICS` (can draw), `COMPUTE`,
`TRANSFER` (can copy). You need the index of one with `GRAPHICS`.

**logical device** — `VkDevice`: your session with the chosen GPU (step 3). Almost
every later object is created from it and must be destroyed before it. "Logical"
because it's your software view of the physical one; several programs can each have
their own logical device on the same GPU.

**handle** — what Vulkan gives you for every object: an opaque number, like a file
descriptor in C. `VkDevice`, `VkQueue`, `VkBuffer`… are all handles. Copying a handle
doesn't copy the object — exactly like copying an `int fd`.

**CreateInfo** — the struct pattern for creating anything: `vkCreateInstance` takes a
`VkInstanceCreateInfo`, `vkCreateDevice` takes a `VkDeviceCreateInfo`, and so on.
You fill in a struct describing what you want and pass a pointer to it. Each starts
with `sType` (structure type — which struct this is) and `pNext` (pointer to next — a
chain for extension structs). `ash` fills `sType` for you.

**`VkResult`** — the return code of almost every Vulkan function: `VK_SUCCESS`, or an
error like `VK_ERROR_INCOMPATIBLE_DRIVER`. In `ash` it arrives as a Rust
`Result<T, vk::Result>`, so `?` works.

**`ash`** — the Rust crate you use for Vulkan. Not an acronym: a pun — what's left
after Vulkan's fire. A thin 1:1 binding: `ash::Entry` holds the loaded loader;
`ash::Instance` and `ash::Device` hold the function pointers for that instance and
device. Every Vulkan call through `ash` is `unsafe`.

---

## How it actually works

### The order of operations

```
ash::Entry::load()                 open libvulkan.dylib (the loader)
      │
      ▼
create instance  ◄── app info, instance extensions, layers, portability flag
      │
      ├── create debug messenger   (development builds)
      │
      ▼
enumerate physical devices ──► check each: name, queue families, extensions ──► choose one
      │
      ▼
create logical device  ◄── which queue family, how many queues, device extensions
      │
      ▼
get queue handle  (family index, queue 0)
```

Destruction is the exact mirror: **device → debug messenger → instance**. Queues and
physical devices are not destroyed — they belong to their parents.

### 1. Loading the loader

`ash::Entry::load()` opens `libvulkan.dylib` at runtime. On macOS with Homebrew the
library lives in `/opt/homebrew/lib`. If `Entry::load()` fails with "library not
found", that folder isn't on the search path — the fix is setting
`DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib` (later, in the Makefile's run rule).
*I'm not certain your setup needs this — `vkcube` working is a good sign — so test it.*

### 2. Creating the instance

You fill in, from the inside out:

- **application info** — your app's name and version, the engine name, and the
  **API version** you're targeting. Use Vulkan 1.2 or 1.3; check what
  `vulkaninfo --summary` reports for MoltenVK and don't ask for more.
- **instance extensions** — for now `VK_KHR_portability_enumeration` and
  `VK_EXT_debug_utils`. § 4 adds the window ones.
- **layers** — `VK_LAYER_KHRONOS_validation`, debug builds only
  (`cfg!(debug_assertions)` is true for `cargo build`, false for `--release`).
- **flags** — `ENUMERATE_PORTABILITY_KHR`.

Extension and layer names are C strings. Rust 2021+ has C-string literals
(`c"VK_LAYER_KHRONOS_validation"`), and `ash` provides extension names as constants
(`ash::khr::portability_enumeration::NAME`, `ash::ext::debug_utils::NAME`). You hand
Vulkan an *array of pointers* to them (`.as_ptr()` on each) — § 1's "the owner must
outlive the pointer" rule in practice. Literals and constants live for the whole
program, so they're safe.

### 3. The debug messenger

You provide an `extern "system" fn` — a function with the C calling convention, so the
C driver can call it. It receives a severity (verbose, info, warning, error), a type,
and the message. Print warnings and errors; skip verbose and info at first, because
MoltenVK is chatty. Return `vk::FALSE` ("don't abort the call").

### 4. Choosing the physical device

For each physical device:
- read its properties (name, type, API version);
- list its queue families and find the index of one with `GRAPHICS`;
- (§ 4 adds: can it present to the window, and does it support the swapchain extension?)

Keep the first one that passes. On your Mac: "Apple M-something".

### 5. Creating the logical device

You say: from queue family *N*, I want *1* queue, priority `1.0` (priorities only
matter with several queues). Plus device extensions: for now
`VK_KHR_portability_subset` **if the device lists it** (check with
`enumerate_device_extension_properties`). § 4 adds `VK_KHR_swapchain`.

Then `get_device_queue(family_index, 0)` hands you the queue.

### 6. Owning it all in Rust

One struct — a GPU context — holds the entry, the instance, the debug messenger, the
physical device, the logical device, the queue and the family index. It implements
`Drop` and destroys explicitly, in mirror order. Don't rely on field order (§ 1's
trap): explicit is easier to read and to defend.

---

## Known traps

- **`ERROR_INCOMPATIBLE_DRIVER` when creating the instance**, or "0 physical devices".
  You forgot `VK_KHR_portability_enumeration` or the `ENUMERATE_PORTABILITY_KHR` flag.
  You need both.
- **`ERROR_LAYER_NOT_PRESENT`** — the validation layer isn't installed or isn't
  found. Check `vulkaninfo --summary` lists it. Fall back to running without it (with
  a warning) rather than refusing to start — the evaluation machine may not have it.
- **Validation seems "silent"** — the layer isn't actually enabled (release build, or
  the layer wasn't found and your fallback kicked in), or your callback filters out the
  severity. (Without any messenger, the layer usually logs to stdout on its own — but
  you can't filter or format that.) Test it: skip destroying the device once; you must
  see an error at exit.
- **Validation error about `VK_KHR_portability_subset`** — the device lists it and
  you didn't enable it.
- **Garbled names or random crashes at instance creation** — a dangling pointer to a
  temporary `CString` (§ 1). `c"..."` literals and `ash`'s `NAME` constants avoid it.
- **"objects not destroyed" at exit** — wrong destruction order or a missing destroy.
- **Asking for a newer API version than MoltenVK supports** — creation may still
  succeed, but newer features then fail. Match `vulkaninfo`.

---

## Checkpoint

**Satisfies:** nothing visible yet — this is the base every rendering requirement
stands on. (Method: gate first, then Claude writes it and walks you through it.)

1. `ash` added to the engine, pinned (`ash = "=0.38.0"`).
2. A GPU-context type in the engine that, on creation: loads the loader, creates the
   instance (portability + debug utils, validation in debug builds), creates the debug
   messenger, picks a physical device with a graphics queue family, creates the logical
   device and gets the queue. On drop: destroys in mirror order.
3. Prints the GPU name, its API version and the chosen queue family index.
4. Created from scop **before** the window opens, for now (the window joins in § 4).
5. **Prove validation works:** comment out the device destroy, run, see the error,
   put it back.

**Verify:** GPU name printed; no validation messages on a normal run; an error when you
skip a destroy; `cargo clippy` clean.

---

## Questions

Q-3.1 (recall) Name the three things you create in this section, in creation order,
and the order you destroy them in.

Q-3.2 (prediction) You follow a Windows Vulkan tutorial exactly on your Mac. Instance
creation fails, or zero GPUs are found. What's the most likely cause, and which two
things fix it?

Q-3.3 (application) A GPU reports three queue families: family 0 has
`COMPUTE | TRANSFER`, family 1 has `GRAPHICS | COMPUTE | TRANSFER`, family 2 has
`TRANSFER`. Which index do you use, and why?

Q-3.4 (prediction) Your code enables validation only when `cfg!(debug_assertions)`
is true. You run `cargo run --release`, forgetting to destroy a buffer. What do you
see in the terminal, and why?

Q-3.5 (prediction) You copy a `VkDevice` handle into two variables and call
`destroy_device` through each. What happens, and how can Rust ownership stop this
happening in the engine?

Answers: `ANSWERS.md` § 3
