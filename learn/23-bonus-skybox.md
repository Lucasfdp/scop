---
section: 23
slug: bonus-skybox
tier: T1
status: in-progress
prereqs: [11, 14, 20, 22]
terms: [cube map, cube face, direction lookup, skybox, depth writing off, environment map, reflection, chrome, generated sky]
---

# Section 23 — A sky around the scene (bonus)

**Depth: T1, no maths (as §§ 20 and 22).** What each piece does, where it lives, how to see it's
right; the few formulas are GLSL built-ins or given in the code.
Bonus extra (owner's pick, 2026-11-18): **a cube-map skybox, plus a chrome (reflective) mode**.
**You'll be able to:** surround the model with a sky that turns as you orbit, and make the model
a mirror that reflects it.

---

## The idea

**A picture all the way round.** A normal texture (§ 14) is one flat picture, looked up with
(u, v). A **cube map** is six square pictures — the inside faces of a box: right, left, up,
down, front, back — stored as **one** texture. You don't look it up with (u, v) but with a
**direction**: "what's over there?" The GPU picks the face that direction points at and the
spot on it. Any direction gives an answer, so it covers the whole surroundings.

**The skybox.** Draw a big box around the camera, and colour every pixel of it by looking up
the cube map in the direction from the camera to that pixel. The box moves *with* the camera,
so you can never get closer to a wall or see a corner; only turning (the orbit) changes what
you see. That's what makes a sky feel infinitely far away (Q-23.2).

It's drawn **first**, behind everything, with **depth writing off**: it paints the background
but leaves the depth image (§ 11) at "nothing here yet", so the model always draws over it
however big or small the box is (Q-23.3).

**Chrome.** A mirror shows what's in the direction the view *bounces* off it. For each pixel of
the model: the direction from the camera to that point, bounced off the face (its normal, § 20)
— GLSL has this built in: `reflect` — then the same cube-map lookup. The model now shows the sky
around it, and it changes as you orbit or turn it. scop's faces are flat (§ 20), so each face is
a flat mirror: crisp jumps at the edges, a disco-ball teapot (Q-23.4).

**Which sky.** No downloads, no new image format: scop **generates** its sky at start — a blue
gradient overhead, a pale horizon, darker ground, and a sun placed exactly where the § 20 light
comes from, so the shading and the sky agree (Q-23.5). If six `.tga` faces are in
`resources/sky/`, scop uses those instead.

**Where the analogy stops being true:** a real mirror reflects the model too (the teapot's spout
would show in its body). A cube-map reflection only shows the *sky*: the model never sees itself.

---

## The names for it

**cube map** — six square pictures, one per inside face of a box, stored as one texture and looked
up with a direction.

**cube face** — one of the six: +x (right), −x (left), +y (up), −y (down), +z, −z. Vulkan stores
them as 6 layers in that order.

**direction lookup** — sampling a cube map with a direction (`samplerCube`): the GPU chooses the face
and the spot. The direction's length doesn't matter.

**skybox** — a big box drawn around the camera, moving with it, coloured from the cube map: the sky.

**depth writing off** — a pipeline setting: test against the depth image but don't write into it. The
sky paints the background without blocking anything.

**environment map** — a cube map used for what surrounds the scene (sky and reflections). scop binds
one for every draw (set 0).

**reflection** — the view direction bounced off a surface (`reflect` in GLSL) — where a mirror "looks".

**chrome** — colour mode 4 (C): every pixel shows the sky in its reflection direction.

**generated sky** — scop's built-in cube map, computed at start: gradient, horizon, ground, sun.

---

## How it actually works

### 1. Engine: cube maps, the environment, a background pipeline

```
Gfx::create_cube_texture([&Image; 6])     // 6 square faces, same size, order +x −x +y −y +z −z
    one image, 6 layers, "cube compatible"; view type CUBE; same sampler as § 14
set 0 binding 2: the environment (samplerCube), every draw can read it
    Gfx::set_environment(cube)             // waits for the GPU, points every slot at it
    default: a 1×1 white cube, so it's always valid
camera uniform: + the camera's position (vec4)        // 80 → 96 bytes; chrome needs it
pipeline setting: depth writing off                    // Gfx::create_background_pipeline
```

### 2. scop: the sky

```
generated sky: for each of the 6 faces, each texel → its direction → a colour
               (gradient by how high it points; sun where the light comes from)
               — or resources/sky/{px,nx,py,ny,pz,nz}.tga if all six are there
sky mesh:      a cube, drawn first with the background pipeline
sky.vert:      box centred on the camera (model = move to the camera × make it big)
               direction = the corner's own position (the box is centred on the camera)
sky.frag:      colour = environment lookup in that direction
E: sky on / off (off → the plain background, as before)
```

### 3. scop: chrome

```
mesh.vert: also passes the corner's world position (model × position)
mesh.frag: mode 4 (C):  view = world position − camera position
                        bounce = reflect(view, normal)
                        colour = environment lookup in `bounce`
           unlit (the sky *is* its lighting), like facing / depth (§ 19)
hot reload: watches sky.vert / sky.frag too
```

### Decisions

| Choice | Over | Why |
|---|---|---|
| **Box centred on the camera, drawn first, depth writing off** | Full-screen triangle working out each pixel's direction | No new maths: the box's own corners are the directions; reuses the push block and camera |
| **Generated sky, optional six `.tga`** | Downloading a sky (PNG/HDR) | Our TGA loader only; nothing to find or license; the sun matches the light |
| **Environment in set 0 (per frame, every draw)** | A second texture in set 1 | Chrome needs the picture *and* the sky at once; set 1 stays "this draw's picture" |
| **Camera position in the uniform** | Working it out from the view matrix in the shader | One line in the camera block; no matrix inverse |
| **Chrome as colour mode 4 (C)** | A separate key | It's another way to colour the model, like facing and depth |
| **Flat normals kept** | Smooth normals for chrome | Same mesh as everything else; faceted mirrors are honest for these files |

---

## Known traps

- **Sun on the wrong side, or the sky upside down / mirrored** — the six faces in the wrong order, or a
  face's picture flipped. Vulkan's order is +x, −x, +y, −y, +z, −z, each with its own "up". Check: with
  B on, the bright side of the model should face the sun.
- **Model invisible behind the sky** — the sky pipeline writes depth (Q-23.3), or it's drawn *after* the
  model with the depth test off.
- **Sky cut off at the corners** — the box reaches past the far plane (§ 11): keep it inside.
- **Validation error: image view type** — a cube view needs an image created "cube compatible" with 6
  layers, and square faces of one size.
- **Chrome looks like the light, not the sky** — the light code still applied to mode 4; or the
  environment isn't bound (the white default shows: everything white).
- **Reflection stuck as you orbit** — the camera position not updated, or `view` from a fixed point.
- **Hot reload ignores the sky** — the watch only knew `mesh.*`; it now watches all four shaders.

---

## Checkpoint

**Satisfies (subject):** bonus extra (skybox, chrome). Method: gate first, then Claude writes it and walks
you through it.

1. **Engine:** `create_cube_texture`; set 0 binding 2 environment + `set_environment` + white default;
   camera block + position (96 bytes); `create_background_pipeline` (depth writing off).
2. **scop:** generated sky (and the six-`.tga` option); sky mesh, `sky.vert`/`sky.frag`, drawn first;
   E toggles; chrome as C's 4th mode; world position from `mesh.vert`; hot reload watches all four
   shaders; controls, README.
3. **Tests:** cube faces must be square and the same size; the generated sky is bright up, dark down,
   brightest towards the sun (in the light's direction); camera bytes 96 with the position after the
   light; the sky box fits inside the far plane.

**Verify:** E — sky around the logo, turning as you orbit; B — the lit side faces the sun; C to chrome —
the logo mirrors the sky, changing as you orbit; teapot chrome — a disco ball; crowd (G) in chrome;
E off — the plain background, as before; validation silent.

---

## Questions

Q-23.1 (recall) What is a cube map, and why is it looked up with a direction instead of (u, v)?

Q-23.2 (application) The sky box moves with the camera. What would you see while orbiting if it stayed
where it was instead?

Q-23.3 (prediction) You make the sky box small — 1 unit across, still centred on the camera — and draw it
first with depth writing **on**. What do you see? Why does depth writing off fix it?

Q-23.4 (prediction) In chrome mode, why does the 42 logo show sharp jumps in the reflection at the edges
between faces, and the teapot look like a disco ball?

Q-23.5 (application) Why place the generated sun exactly where the § 20 light comes from? What would look
wrong if it were somewhere else?

Answers: `ANSWERS.md` § 23
