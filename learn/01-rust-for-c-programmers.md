---
section: 1
slug: rust-for-c-programmers
tier: T2
status: not-started
prereqs: []
terms: [ownership, move, borrow, lifetime, Option, Result, ? operator, trait, Drop, RAII, crate, Cargo workspace, unsafe, raw pointer, CString, repr(C)]
---

# Section 1 — Rust for a C programmer

**Depth: T2** — every later section is written in Rust, and Vulkan's object
lifetimes map directly onto Rust's ownership rules. A shaky model here turns every
graphics bug into a fight with the compiler.
**You'll be able to:** read and write the Rust you need for scop without the
borrow checker hiding the actual graphics problem, and set up the workspace the
whole project lives in.

**Read alongside:** *The Rust Programming Language* ("the Book"), chapters 1–10
(free at doc.rust-lang.org/book). This section is the map; the Book is the territory.
Rust *syntax* is not taught here — the Book does that well. This section covers
the ideas that are different from C, and the ones Vulkan will lean on.

---

## The idea

In C, every pointer is a promise you make to yourself: "this memory is still
alive, and nobody else is changing it right now". The compiler believes you. When
you're wrong, you find out at runtime — a segfault, a double free, or worse, a
quietly corrupted value.

Rust moves those promises into the type system. Every value has exactly **one
owner** — one variable responsible for it. When the owner goes out of scope, the
value is cleaned up automatically, like a `free()` the compiler writes for you at
the closing brace. If you hand the value to someone else, ownership *moves*: the
old variable can't be used any more, so there's no way to free it twice.

You can also *lend* a value without giving it away. A lend is either:
- **many readers at once**, or
- **exactly one writer, and no readers.**

Never both. That single rule is what makes data races and "someone changed it
under me" bugs impossible in normal Rust code — and it's checked when you compile,
not when you run.

Errors work differently too. Instead of returning `-1` and setting `errno`, a Rust
function that can fail returns a value that is *either* the result *or* the
error, and the compiler won't let you use the result without deciding what to do
about the error.

When you genuinely need C-style freedom — and talking to Vulkan, a C API, you
will — you write `unsafe { ... }`. That doesn't switch the rules off; it marks
a small block where *you* take back responsibility for the promises, exactly as in C.

---

## The names for it

**ownership** — the rule that every value has exactly one variable responsible
for cleaning it up.

**move** — handing ownership to another variable or function. After a move, the
original variable is unusable. For simple copyable types (`i32`, `f32`, `bool`) a
*copy* happens instead and both stay usable.

**borrow** — lending a value without transferring ownership. `&T` is a *shared
borrow* (read-only, many allowed); `&mut T` is a *mutable borrow* (read-write, only
one, and no shared borrows at the same time). The part of the compiler that checks
this is the **borrow checker**.

**lifetime** — how long a borrow is valid. The compiler usually works it out; when
it can't, you write it as `'a`. The rule: a borrow can never outlive the value it
points to — so dangling pointers can't compile.

**`Option<T>`** — "maybe a value": either `Some(value)` or `None`. Replaces NULL.

**`Result<T, E>`** — "a value or an error": either `Ok(value)` or `Err(error)`.
Replaces return codes and `errno`.

**`?` operator** — written after an expression that returns a `Result`: "if this is
an error, return it from my function right now; otherwise give me the value".
The Rust equivalent of `if (ret < 0) return ret;`, in one character.

**trait** — a set of functions a type promises to provide (like an interface).
`Debug` (printable with `{:?}`), `Clone`, `Drop` are all traits.

**`Drop`** — the trait whose single function `drop()` runs automatically when a
value's owner goes out of scope. Your destructor.

**RAII** — **R**esource **A**cquisition **I**s **I**nitialisation. A clumsy name from
C++ for a simple idea: tie a resource (memory, a file, a Vulkan object) to the
lifetime of a value, so creating the value acquires it and dropping the value
releases it. The name describes the first half and forgets the important second
half — don't try to derive meaning from it.

**crate** — Rust's unit of compilation: a library or a binary. Roughly "one
`.a` or one executable".

**Cargo workspace** — several crates in one repository sharing one build
directory and one lock file. **Cargo** is Rust's build tool and package manager
(think `make` + a package manager in one).

**`unsafe`** — a block or function where you may do five things the compiler
can't check: dereference raw pointers, call `unsafe` functions (all Vulkan calls),
access mutable statics, implement `unsafe` traits, access `union` fields.

**raw pointer** — `*const T` / `*mut T`. Exactly a C pointer: no ownership, no
lifetime, can be null or dangling. Creating one is safe; *using* one is `unsafe`.

**`CString`** — an owned string with a trailing `\0`, laid out like a C `char *`.
Rust's normal `String` is *not* null-terminated, so you need this whenever a C API
(Vulkan) wants a name.

**`repr(C)`** — **repr**esentation C. An attribute (`#[repr(C)]`) that forces a
struct's fields into C's layout and order. Without it Rust may reorder fields.
You'll need it for anything the GPU reads byte-by-byte (vertices, uniforms).

---

## How it actually works

### 1. Moves vs copies

```rust
// THROWAWAY — not part of scop
let a = vec![1, 2, 3];   // a owns a heap buffer
let b = a;               // ownership moves to b; no copy of the buffer
// println!("{:?}", a);  // compile error: borrow of moved value: `a`
println!("{:?}", b);     // fine
```

In C, `b = a` with a pointer would give two pointers to one buffer, and two
`free()`s later you'd have a double free. Rust makes `a` unusable instead.

Types that are cheap and own nothing (`i32`, `f32`, `[f32; 3]`, your own structs
marked `#[derive(Clone, Copy)]`) are copied instead of moved. Your future `Vec3`
will be `Copy`. A Vulkan wrapper struct must **not** be — copying it would mean two
owners destroying the same GPU object.

### 2. Borrowing: the "one writer XOR many readers" rule

```rust
// THROWAWAY — not part of scop
let mut v = vec![1, 2, 3];
let first = &v[0];       // shared borrow
v.push(4);               // error: cannot borrow `v` as mutable because it is also borrowed as immutable
println!("{first}");
```

Why is this an error? `push` might reallocate the buffer, which would leave
`first` pointing at freed memory. In C this compiles and becomes a use-after-free.
Rust refuses because `first` is still in use on the last line.

Function signatures tell you which kind of borrow they need:
- `fn len(&self)` — reads
- `fn push(&mut self, x)` — writes
- `fn into_bytes(self)` — takes ownership (consumes the value)

### 3. Errors: `Result` and `?`

```rust
// THROWAWAY — not part of scop
use std::fs;

fn read_config(path: &str) -> Result<String, std::io::Error> {
    let text = fs::read_to_string(path)?;   // on error: return Err(...) from read_config
    Ok(text.trim().to_string())
}
```

`?` only works inside a function that itself returns a `Result` (or `Option`). In
`main`, you can return `Result<(), Box<dyn std::error::Error>>` so `?` works there
too — `Box<dyn Error>` means "any error type, on the heap".

`.unwrap()` means "crash if this is an error". Fine in quick experiments; in scop,
a missing `.obj` file must produce a clean message, not a panic.

### 4. `Drop` and order of destruction — this is where Vulkan bites

When a struct is dropped, Rust drops its **fields in the order they are declared**,
top to bottom. Local variables are dropped in **reverse** order of declaration.

Vulkan objects have strict destruction order: you must destroy a swapchain before
the device that created it, and the device before the instance. If a Rust struct
holds `instance` declared above `device`, and you rely on automatic drops, the
instance goes first — and the validation layer (§ 3) will shout at you.

You'll handle this in § 3 with an explicit `Drop` implementation that destroys
things in the right order. For now, just remember: **drop order is declaration
order for fields, reverse order for locals.**

### 5. `unsafe` and raw pointers (T1 — you know pointers already)

Every `ash` function that talks to Vulkan is `unsafe`, because Vulkan takes raw
pointers and the compiler can't check them. The pattern you'll use constantly:

```rust
// THROWAWAY — not part of scop
use std::ffi::CString;

let name = CString::new("scop").unwrap();   // owned, null-terminated
let ptr: *const std::os::raw::c_char = name.as_ptr(); // borrowed raw pointer
// ... hand ptr to a C API while `name` is still alive ...
```

The rule you must enforce yourself: **the owner must outlive every raw pointer to
it.** The borrow checker does not track raw pointers.

Keep `unsafe` blocks small and wrap them: a safe function that does the unsafe call
correctly, so the rest of the code never sees a raw pointer. That's the backbone of
the engine crate.

### 6. Crates, modules and the workspace

```
scop/
├── Cargo.toml            # [workspace] members = ["engine", "scop"]
├── engine/               # library crate — reusable in ft_newton, ft_vox...
│   ├── Cargo.toml
│   └── src/lib.rs
└── scop/                 # binary crate — the 42 project itself
    ├── Cargo.toml        # depends on engine = { path = "../engine" }
    └── src/main.rs
```

- `mod foo;` in `lib.rs` pulls in `src/foo.rs` (or `src/foo/mod.rs`).
- Everything is **private by default**; `pub` makes it visible outside the module.
  That's your tool for keeping the engine's public API small.
- `cargo build` at the root builds everything; `cargo run -p scop -- args` runs
  the binary with arguments.

---

## Known traps

- **`error[E0382]: borrow of moved value`** — you used a variable after moving it.
  Either borrow (`&x`) instead of moving, or `.clone()` if you truly need two copies.
- **`error[E0502]: cannot borrow ... as mutable because it is also borrowed as immutable`**
  — a shared borrow is still alive where you try to mutate. Usually fixed by
  finishing with the read before the write, or by copying the value out first.
- **Dangling `CString` pointer** — `CString::new("x").unwrap().as_ptr()` in one
  expression creates a temporary that is dropped at the end of the statement. The
  pointer is dangling immediately. Compiles fine, crashes or garbles at runtime.
  Always bind the `CString` to a variable first.
- **Self-referential structs** — a struct holding both a value and a reference into
  that value won't compile. If you hit this, the design is the problem: store
  owned values or indices, not references to siblings.
- **Fighting the borrow checker with `.clone()` everywhere** — a sign that ownership
  isn't decided. Ask "who owns this?" before adding a clone.
- **`unwrap()` on user input** — a missing file panics with a backtrace. The
  evaluator will try a wrong path.

---

## Checkpoint

Build the skeleton the whole project lives in. **Satisfies:** project structure,
and the start of "parse a `.obj` file" (reading it).

1. Create the workspace exactly as in "How it actually works" § 6: a root
   `Cargo.toml` with `[workspace]`, an `engine` library crate, a `scop` binary crate
   that depends on `engine`.
2. In `engine`, write a public function that takes a path and returns
   `Result<String, SomeErrorType>` with the file's contents. Choose your error type
   (`std::io::Error` is fine for now).
3. In `scop`, read the path from the command-line arguments (`std::env::args`),
   call the engine function, and:
   - print the number of lines on success,
   - print a clean one-line error and exit with a non-zero code on failure —
     **no panic, no `unwrap()`** on the user's input,
   - print a usage message if no argument is given.
4. Run `cargo clippy` and fix every warning.

**Verify:** `cargo run -p scop -- resources/42.obj` prints a line count;
`cargo run -p scop -- nope.obj` prints an error and `echo $status` (fish) shows
non-zero; `cargo run -p scop` prints usage.

---

## Questions

Q-1.1 (recall) What is the difference between `&T` and `&mut T`, and how many of
each may exist at the same time for one value?

Q-1.2 (prediction) This compiles in C and crashes at runtime. Does the Rust version
compile? If not, which rule stops it, and what bug in C does that rule prevent?
```rust
let mut names = vec![String::from("cube")];
let first = &names[0];
names.push(String::from("teapot"));
println!("{first}");
```

Q-1.3 (prediction) A struct is declared as
`struct Gpu { instance: Instance, device: Device }`, both with `Drop`
implementations that destroy the Vulkan object. When a `Gpu` is dropped, which is
destroyed first? Why is that a problem for Vulkan?

Q-1.4 (application) Your function returns `Result<Mesh, String>`. Inside it you call
`fs::read_to_string(path)`, which returns `Result<String, std::io::Error>`. Why
won't `fs::read_to_string(path)?` compile as-is, and what is one way to fix it?

Q-1.5 (prediction) What is wrong with this line, even though it compiles?
```rust
let p = CString::new("scop").unwrap().as_ptr();
```

Answers: `ANSWERS.md` § 1
