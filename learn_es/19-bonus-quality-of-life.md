---
section: 19
slug: bonus-quality-of-life
tier: T1
status: done
prereqs: [16, 18]
terms: [frames per second, frame time, vsync cap, averaging window, title throttling, dropped file, runtime mesh replacement, stale handle, debug view, gl_FrontFacing, winding order, inside-out face, linear depth, push params]
---

# Sección 19 — Comodidades: FPS, arrastrar y soltar, colores de depuración (bonus)

**Profundidad: T1** — sobre todo reutilización: los fotogramas en vuelo de § 5, el
`device_wait_idle` de § 16, la `w` de § 9, el bloque de push de § 15. Extras del bonus
(elegidos por el propietario, 2026-11-18): **FPS en el título**, **soltar un `.obj` en la
ventana para cargarlo**, **modos de color de depuración (orientación / profundidad)**.
**Serás capaz de:** ver a qué velocidad corre scop de verdad, cambiar de modelo sin
reiniciar, y mirar *dentro* de la imagen que hace la GPU: hacia dónde mira cada cara y a qué
distancia está.

---

## La idea

**FPS.** Fotogramas por segundo: cuántas imágenes termina scop cada segundo. El engine ya
sabe `dt` (segundos desde el último fotograma), así que `1 ÷ dt` es "los FPS ahora mismo" —
pero salta en cada fotograma y es ilegible. En su lugar, cuenta fotogramas durante **medio
segundo** y luego muestra `fotogramas ÷ tiempo`. El número cambia dos veces por segundo y
significa algo.

Hay un techo con el que te toparás enseguida: scop presenta con **FIFO** (§ 4), que espera al
refresco de la pantalla. En una pantalla de 60 Hz scop no puede mostrar más de 60 fotogramas
por segundo, en un MacBook de 120 Hz 120 — por pequeño que sea el modelo. Tocar el techo
significa "al menos tan rápido como la pantalla", no "este es el límite de la GPU". El
**frame time** (milisegundos por fotograma) es el mismo número dado la vuelta, y es el que
comparan los programadores: 60 fps = 16.7 ms.

**Arrastrar y soltar.** El sistema de ventanas le dice a la app "te han soltado un archivo
encima" — un evento más, como una tecla. scop lo lee y lo parsea igual que el modelo de la
línea de comandos. Si está roto, scop imprime el error y **se queda con el modelo viejo**:
soltar un archivo malo nunca cierra el visor.

Cambiar de modelo en plena ejecución es el problema nuevo. Hasta ahora las meshes se creaban
una vez, en `setup`. Ahora se crean y se **destruyen** mientras hay fotogramas en vuelo — y
puede que la GPU siga dibujando el modelo viejo en un fotograma enviado hace un momento
(§ 5: hasta dos fotogramas). Destruir entonces sus buffers es un use-after-free en la GPU. El
arreglo de scop es el que ya conoces: `device_wait_idle` (§ 16) antes de destruir. Una pausa
de un fotograma, solo cuando se suelta un archivo.

**Colores de depuración.** Sustituir la imagen por colores que *significan* algo:

- **Orientación:** pintar el lado de cada triángulo que estás viendo — delante = azul, detrás
  = rojo. Un modelo cuyas caras miran al revés ("del revés") se ve rojo por fuera.
- **Profundidad:** pintar cada píxel según lo lejos que esté de la cámara — cerca = blanco,
  lejos = oscuro. Ves la forma del modelo en profundidad, lo mismo que compara el depth buffer
  (§ 11).

C va rotando: normal → orientación → profundidad.

**Dónde deja de valer la analogía:** "delante" no es algo que un triángulo *tenga*. La GPU lo
decide por el orden en que caen las tres esquinas en pantalla (horario o antihorario) — así
que cualquier cosa que refleje la imagen (como la inversión de y de la tarjeta de perspectiva
de § 9) intercambia delante y detrás salvo que el ajuste `front_face` del pipeline lo tenga
en cuenta.

---

## Los nombres

**frames per second (FPS)** — fotogramas terminados por segundo, promediados en una ventana
corta de tiempo.

**frame time** — milisegundos por fotograma: `1000 ÷ FPS`. Se suma honestamente (dos
trabajos de 8 ms = 16 ms); los FPS no.

**vsync cap** — la presentación FIFO espera al refresco de la pantalla, así que los FPS no
pueden pasar de la frecuencia de refresco (60, 120 …). Los otros present modes (MAILBOX,
IMMEDIATE) quitan el techo.

**averaging window** — cuánto tiempo se cuentan fotogramas antes de actualizar el número
(0.5 s aquí).

**title throttling** — cambiar el título de la ventana solo cuando cambia su texto. Ponerlo
cada fotograma es lento en algunos sistemas (macOS redibuja la barra de título cada vez).

**dropped file** — el `WindowEvent::DroppedFile(path)` de winit: un evento por archivo
soltado.

**runtime mesh replacement** — crear y destruir meshes después de `setup`, mientras hay
fotogramas en vuelo. Necesita `Gfx` durante `update`, y una espera antes de destruir.

**stale handle** — un `MeshId` cuya mesh se destruyó. Dibujar con él debe ser un error que
reporte el engine, nunca un crash de la GPU: el hueco se vacía, no se reutiliza.

**debug view** — colores falsos que muestran un número oculto (orientación, profundidad,
normales, UVs…).

**gl_FrontFacing** — entrada del fragment shader: `true` si el triángulo se ve por delante.
Siempre `true` para puntos y líneas (no tienen lados).

**winding order** — si las esquinas de un triángulo van en sentido horario o antihorario *en
pantalla*. El `front_face` del pipeline dice cuál cuenta como delante (scop:
`COUNTER_CLOCKWISE`).

**inside-out face** — una cara cuyas esquinas están listadas en sentido horario vistas desde
fuera: su delante apunta hacia dentro del modelo.

**linear depth** — la distancia a la cámara en la dirección de la vista. Tras la tarjeta de
perspectiva es simplemente `w` (§ 9: "la última fila copia la distancia en w").

**push params** — cuatro números definidos por la app en el bloque de push (§ 15), junto a la
matriz de modelo: scop pone ahí el fundido, el modo de depuración y el rango de profundidad.

---

## Cómo funciona realmente

### 1. FPS en el título (engine + scop)

```
engine: FpsCounter { frames, elapsed }      // un pequeño helper público
        tick(dt): frames += 1, elapsed += dt
                  elapsed ≥ 0.5 → informar (frames / elapsed, elapsed / frames × 1000), reiniciar
        Frame::set_title(text)              // la app pide un título
        platform: window.set_title solo si text ≠ último texto   (title throttling)
scop:   title = "scop — 42.obj — 120 fps (8.3 ms)"
```

### 2. Arrastrar y soltar (engine + scop)

```
engine: WindowEvent::DroppedFile(path) → Input::dropped_files() de este fotograma
        App::update ahora recibe también  gfx: &mut Gfx   (crear meshes en plena ejecución)
        Gfx::destroy_mesh(id): device_wait_idle, destruir buffers, vaciar el hueco
        meshes: Vec<Option<Mesh>>  → dibujar un id destruido = GpuError::UnknownHandle
scop:   .obj soltado → leer → parsear → (¿error? imprimir, quedarse el modelo viejo)
                     → meshes de los objetos nuevos → destruir las viejas → nuevo centro, radio
                     → reiniciar colocación y zoom; imágenes y modo de vista se mantienen
        se suelta otra cosa → "scop: x.png: drop a .obj file"
```

### 3. Colores de depuración (bloque de push del engine + shaders de scop)

```
bloque de push: mat4 model | vec4 params          // 64 + 16 = 80 bytes (≤ 128, siempre permitido)
                params = (blend, mode, depth_near, depth_far)   — significado de scop, no del engine
engine:     Frame::draw_with(pipeline, mesh, texture, model, params: [f32; 4])
            draw_textured = draw_with(…, [blend, 0, 0, 0])   (sin cambios para quien lo llama)
mesh.vert:  out distance = gl_Position.w                    // profundidad lineal (§ 9)
mesh.frag:  modo 0: mix(grey, picture, blend)              // como antes
            modo 1: gl_FrontFacing ? azul : rojo
            modo 2: t = clamp((distance − near) / (far − near)); grey = 1 − t
scop:       rango de profundidad = el propio del modelo: distancia de la cámara − position.z ± radius
            C: mode = (mode + 1) % 3
```

El modo profundidad usa `w`, **no** `gl_FragCoord.z` (la profundidad guardada). La
profundidad guardada queda apretada hacia 1 por la tarjeta de perspectiva: con el plano
cercano diminuto de scop, todo el modelo cae más o menos en 0.99–1.0 — una mancha blanca
(Q-19.5). La distancia repartida sobre el propio rango del modelo usa toda la escala de
blanco a oscuro.

### Decisiones

| Elección | Frente a | Por qué |
|---|---|---|
| **FPS promediados en 0.5 s**, más frame time | `1 ÷ dt` cada fotograma | Legible; el frame time es lo que se compara |
| **`FpsCounter` en el engine, título elegido por la app** | El engine escribe el título él mismo | Toda app tiene el contador; cada una decide qué dice su título |
| **`update` recibe `&mut Gfx`** | Un callback aparte de "archivos soltados" | ft_vox (chunks) y ft_newton (objetos que aparecen) también necesitarán meshes en plena ejecución |
| **`device_wait_idle` antes de destruir** | Una cola de borrado (destruir tras `FRAMES_IN_FLIGHT` fotogramas) | Soltar archivos es raro; una pausa es invisible. La cola es para ft_vox, que cambia meshes cada fotograma |
| **`Vec<Option<Mesh>>`, ids nunca reutilizados** | Reutilizar huecos liberados | Un id caducado es un error, nunca la mesh de otro |
| **Params de push vec4, significado lo pone la app** | Campos del engine llamados `debug_mode`, `depth_range` | Mantiene las ideas de scop fuera del engine (regla 6 de CLAUDE.md) |
| **Profundidad a partir de `w`**, rango según el tamaño del modelo | `gl_FragCoord.z` | La profundidad guardada no es lineal y se amontona cerca de 1 |
| **Orientación: delante azul, detrás rojo** | Normales como colores | Responde la única pregunta que importa para los `.obj`: ¿del revés o no? |

---

## Trampas conocidas

- **FPS clavados en exactamente 60 / 120** — el techo de vsync, no un programa lento
  (Q-19.1). Compara frame times en un build *release*: los builds de depuración y las
  validation layers son mucho más lentos.
- **El título parpadea o la app da tirones** — poner el título cada fotograma; ponlo solo
  cuando cambie el texto.
- **Crash o error de validación justo después de soltar un archivo** — meshes viejas
  destruidas mientras un fotograma en vuelo aún las usa (Q-19.3). Espera antes.
- **Modelo soltado invisible o diminuto** — centro y radio siguen siendo los del modelo
  viejo; recalcula los dos.
- **Soltar un archivo roto cierra scop** — un error devuelto desde `update` en vez de
  imprimirlo. Un archivo malo es un error del usuario, no un motivo para salir.
- **Error de validación "push constant range"** — el tamaño del bloque de push cambió en los
  shaders pero no en el pipeline layout (o al revés): de 68 a 80 bytes en todas partes a la
  vez (§ 15, Q-15.4).
- **Todo rojo en modo orientación** — o el archivo está del revés, o la inversión de y
  intercambia delante y detrás (Q-19.4). Prueba con un modelo de confianza antes de culpar al
  archivo.
- **Puntos todos azules en modo orientación** — esperado: un punto no tiene lados, así que
  siempre cuenta como delante. (El wireframe sigue mostrando los dos colores: sus líneas
  salen de triángulos, que sí tienen lados.)
- **Modo profundidad todo blanco** — se usó `gl_FragCoord.z`, o el rango no sigue el
  movimiento de W/S.

---

## Checkpoint

**Cumple (subject):** extras del bonus (FPS, arrastrar y soltar, colores de depuración).
Método: primero el gate, luego Claude lo escribe y te lo explica.

1. **Engine:** `FpsCounter`; `Frame::set_title` con throttling; `Input::dropped_files()`;
   `App::update(…, gfx)`; `Gfx::destroy_mesh` (esperar, destruir, vaciar el hueco); bloque
   de push `model + vec4 params`, `Frame::draw_with`.
2. **scop:** título `scop — <archivo> — <fps> fps (<ms> ms)`; soltar un `.obj` → cargar o
   informar; C rota normal / orientación / profundidad; `distance` desde el vertex shader;
   rango de profundidad según el modelo; lista de controles y README actualizados.
3. **Tests:** `FpsCounter` promedia y se reinicia; el título solo cambia cuando cambia el
   texto; un `MeshId` caducado es un error; los bytes de push son 80 con los params tras la
   matriz; soltar un archivo roto conserva el modelo viejo (la función de carga devuelve un
   error, el estado no cambia).

**Verificar:** ejecuta un build *release* — el título muestra FPS a la frecuencia de refresco
de tu pantalla; suelta `teapot.obj`, luego `broken.obj` (error impreso, la tetera se queda),
luego `concave.obj`; C sobre el logo: en orientación debería verse todo azul desde fuera (si
no, averigua cuál es la causa — Q-19.4), en profundidad blanco delante fundiéndose a oscuro;
validación callada en todo momento.

---

## Preguntas

Q-19.1 (recuerdo) El título dice exactamente 120 fps tanto si el modelo es el logo de 42
como si es la tetera. ¿Qué lo está limitando, y qué te dice ese número — y qué no te dice?

Q-19.2 (aplicación) ¿Por qué promediar los FPS durante medio segundo en vez de mostrar
`1 ÷ dt` cada fotograma? Da dos razones.

Q-19.3 (predicción) Sueltas un modelo nuevo, y scop destruye los buffers del modelo viejo
enseguida, sin esperar. ¿Qué puede salir mal, cuándo, y te lo diría la validación?

Q-19.4 (predicción) En modo orientación el logo de 42 sale rojo en todas las caras
exteriores. Da dos causas posibles, y una prueba rápida que las distinga.

Q-19.5 (aplicación) ¿Por qué colorear según `gl_FragCoord.z` (la profundidad guardada)
pintaría el modelo entero casi de blanco, y qué usa scop en su lugar?

Respuestas: `ANSWERS.md` § 19
