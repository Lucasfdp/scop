---
section: 15
slug: smooth-transition
tier: T1
status: done
prereqs: [14]
terms: [blend factor, linear interpolation, target value, transition duration, easing, smoothstep, reversible transition, push constant range for two stages]
---

# Sección 15 — Fundir el color en la textura

**Profundidad: T1** — un número, una tecla, una línea de shader. Las ideas de fondo (animar
hacia un objetivo, suavizar la entrada y la salida) están por todas partes en juegos e
interfaces, así que vale la pena tenerlas claras, pero no hay maquinaria nueva de GPU.
**Serás capaz de:** pulsar T para fundir suavemente de las caras grises a la imagen,
pulsarla otra vez (incluso a mitad) para volver, sin ningún salto.

---

## La idea

El subject: *"pulsar una tecla concreta debe aplicar una textura al objeto. Pulsar esa misma
tecla de nuevo debe devolverlo a la vista en color. Se espera una transición suave entre
ambas, sin cortes bruscos."*

Ahora mismo cada píxel tiene dos colores posibles: el gris de su cara (§ 13) y su punto en
la imagen (§ 14). Una transición es solo **una mezcla de los dos**, controlada por un número
de 0 a 1: a 0 todo gris, a 1 todo imagen, a 0.3 "70 % gris, 30 % imagen". Desliza ese número
de 0 a 1 en medio segundo y la imagen aparece fundiéndose.

Piensa en un regulador de luz en vez de un interruptor. La tecla no *pone* la luz; dice
hacia qué lado girar el regulador. Cada fotograma, el regulador avanza un pasito hacia donde
le han dicho que vaya. Pulsa la tecla otra vez a mitad, y el regulador simplemente empieza a
girar de vuelta desde donde esté — sin salto, porque nada teletransporta nunca la rueda.

Un toque más: un regulador girado a velocidad constante arranca y para de golpe. Suavizarlo
— lento al principio, rápido en medio, lento al final — es lo que hace que un fundido se
sienta suave en vez de mecánico.

**Dónde deja de valer la analogía:** un regulador tiene una sola rueda para toda la
habitación. Aquí el número se envía con cada draw (otra vez el pósit), así que en una escena
con varios modelos cada uno podría fundirse por su cuenta.

---

## Los nombres

**blend factor** — el número 0..1 que dice cuánto del segundo color usar. Aquí: cuánta
imagen (frente a gris).

**linear interpolation** — `a + (b − a) × t`: el punto a una fracción `t` del camino de `a`
a `b`. A menudo llamada **lerp**; en GLSL la función integrada es `mix(a, b, t)`.

**target value** — hacia dónde va el blend factor: 0 (gris) o 1 (imagen). La tecla T
invierte el objetivo, nunca el blend factor en sí.

**transition duration** — cuánto dura un fundido completo. scop: 0.6 s. El blend factor se
mueve `dt / duration` por fotograma, limitado a 0..1.

**easing** — remodelar el progreso 0..1 para que el movimiento empiece y termine con
suavidad en vez de bruscamente. La entrada es el progreso en crudo; la salida es lo que se
usa de verdad.

**smoothstep** — la curva de suavizado clásica: `t² × (3 − 2t)`. En 0 y en 1 su pendiente
es cero, así que el fundido empieza y para sin "tirón". GLSL la tiene integrada; scop la
aplica en la CPU.

**reversible transition** — una que puede cambiar de dirección en cualquier momento sin
salto. Sale gratis de "mover el valor hacia un objetivo" en vez de "reiniciar una
animación".

**push constant range for two stages** — el rango de § 10 crece de "vertex shader, 64
bytes" a "vertex **y fragment** shaders, 68 bytes": la matriz de modelo más un float. Los
dos shaders declaran el mismo bloque; cada uno usa su parte.

---

## Cómo funciona realmente

### 1. Estado y teclas (scop)

```
estado:  texture_target: f32   (0 o 1)
         texture_progress: f32 (0..1, en crudo, lineal)

update:  if pressed(T): texture_target = 1 − texture_target       (pressed: un interruptor, Q-13.3)
         step = dt / FADE_SECONDS
         mover texture_progress hacia texture_target como mucho `step`
```

"Hacia, como mucho `step`" significa: si está por debajo del objetivo, suma `step` pero sin
pasarse del objetivo; si está por encima, resta. Cuando llega, se queda.

### 2. Suavizar y enviar (scop → engine)

```
draw:    blend = smoothstep(texture_progress)       // t² × (3 − 2t)
         frame.draw_textured(pipeline, mesh, texture, model, blend)
```

### 3. El pósit crece (engine)

```
push constant range:  etapas VERTEX | FRAGMENT, offset 0, tamaño 68
                      bytes 0..64 = model (mat4), bytes 64..68 = blend (float)
cmd_push_constants:   mismas etapas, los 68 bytes
```

### 4. Los shaders

```glsl
// mesh.vert y mesh.frag
layout(push_constant) uniform Push {
    mat4 model;
    float blend;   // § 15: 0 = gris, 1 = imagen
} push;

// mesh.frag
out_colour = mix(vec4(colour, 1.0), texture(picture, uv), push.blend);
```

La mezcla ocurre sobre el brillo real, no sobre números codificados, porque la textura es
`SRGB` (se decodifica al leer) y la swapchain es `SRGB` (se codifica al escribir) — la nota
al pie de § 14: una mezcla al 50 % se ve a mitad de camino.

### Decisión: animación en la CPU, blend en el push constant, smoothstep

| Elección | Frente a | Por qué |
|---|---|---|
| **Animar en la CPU con `dt`, hacia un objetivo** | Un temporizador que arranca al pulsar (reiniciar da saltos; más difícil de invertir a mitad) | Reversible gratis; la misma idea por segundo que en § 13 |
| **Blend factor en el push constant** | En el uniform buffer de la cámara | Es por *draw* (cada modelo podría fundirse por su cuenta), no por fotograma |
| **Suavizado smoothstep** | Fundido lineal sin más | Sin "tirón" al principio ni al final — los "sin cortes bruscos" del subject |

---

## Trampas conocidas

- **La imagen aparece / desaparece de golpe** — la tecla pone directamente el blend factor en
  vez del objetivo; o `pressed` y `held` intercambiados (mantener la tecla lo invierte cada
  fotograma).
- **Un salto al pulsar T a mitad** — el fundido vuelve a empezar desde 0 o 1 en vez de
  moverse desde donde está.
- **La velocidad del fundido depende de la máquina** — el paso no es `dt / duration`.
- **Error de validación al crear el pipeline: push constant range** — el fragment shader lee
  push constants pero los flags de etapa del rango solo dicen `VERTEX` (Q-15.4).
- **Error de validación en `cmd_push_constants`** — sus flags de etapa o su tamaño no
  coinciden exactamente con el rango.
- **El blend se queda un poco por debajo de 0 o 1 / se pasa** — falta el límite al avanzar
  hacia el objetivo.
- **La mitad se ve demasiado oscura o demasiado clara** — mezclar colores codificados (una
  textura `UNORM`); deja las dos `SRGB`.

---

## Checkpoint

**Cumple (subject):** "una tecla concreta aplica la textura y la quita … transición suave,
sin cortes bruscos" — la última funcionalidad obligatoria. Método: primero el gate, luego
Claude lo escribe y te lo explica.

1. **Engine:** rango de push `VERTEX | FRAGMENT`, 68 bytes; `Draw` lleva el blend factor;
   `Frame::draw_textured(…, blend)`; `Frame::draw` usa blend 0 (solo colores).
2. **Shaders:** el bloque de push compartido en los dos; `mix` en el fragment shader.
3. **scop:** objetivo + progreso, T invierte el objetivo, avance `dt / FADE_SECONDS` con
   límite, smoothstep, T añadida a los controles impresos. Tests: el avance llega al
   objetivo y se queda, se invierte a mitad sin salto; los extremos y el centro de
   smoothstep.

**Verificar:**
- `cargo run -p scop -- resources/42.obj`: empieza en gris; T funde a la imagen en ~0.6 s;
  T funde de vuelta; T pulsada a mitad del fundido se invierte con suavidad.
- Validación callada; `cargo test`, `cargo clippy --all-targets` limpios.

---

## Preguntas

Q-15.1 (recuerdo) ¿Qué da `mix(a, b, t)` con `t = 0`, `t = 0.5` y `t = 1`? En scop, ¿qué son
`a`, `b` y `t`?

Q-15.2 (aplicación) ¿Por qué la tecla T invierte un *objetivo* en vez de poner directamente
el blend factor a 0 o 1?

Q-15.3 (predicción) Pulsas T para fundir a la imagen, y la vuelves a pulsar cuando el
fundido va por el 40 %. Describe qué pasa durante el siguiente medio segundo.

Q-15.4 (predicción) Añades `float blend` al bloque de push del fragment shader, pero dejas
las etapas del push constant range solo en `VERTEX`. ¿Qué pasa, y cuándo?

Q-15.5 (aplicación) smoothstep da el mismo principio (0) y final (1) que un fundido lineal.
Entonces, ¿cuál es la diferencia visible, y en qué parte del fundido la ves?

Respuestas: `ANSWERS.md` § 15
