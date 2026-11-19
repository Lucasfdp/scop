---
section: 6
slug: shaders-and-pipeline
tier: T2
status: done
prereqs: [5]
terms: [shader module, entry point, build script, glslc, include_bytes!, SPIR-V magic number, graphics pipeline, fixed-function state, input assembly, viewport, scissor, dynamic state, cull mode, front face, colour write mask, pipeline layout, dynamic rendering, VK_KHR_dynamic_rendering, load op / store op, gl_VertexIndex, gl_Position, location, interpolation]
---

# Sección 6 — Programas que corren en la GPU

**Profundidad: T2** — el pipeline es donde vive la mayoría de bugs de "no sale nada en
pantalla": un ajuste mal y tienes una ventana vacía sin ningún error. Saber para qué sirve
cada ajuste es lo que convierte eso de una tarde adivinando en un minuto comprobando.
**Serás capaz de:** escribir un vertex shader y un fragment shader, compilarlos dentro del
build, cargarlos tú mismo, construir un pipeline gráfico y dibujar tu primer triángulo.

---

## La idea

Hasta ahora la GPU solo hacía una cosa: llenar un lienzo entero de un color plano. Ahora
tiene que *dibujar una forma*. Ya conoces las etapas de § 2 — entran vértices, salen
píxeles. Lo nuevo es que **dos de esas etapas son programas que escribes tú**, y **todo lo
que las rodea es un conjunto de interruptores que pones una vez, por adelantado**.

Piensa en una cadena de producción en una fábrica, haciendo un dibujo:

1. **Puesto 1 — "¿dónde van las esquinas?"** Le das tres números de esquina (0, 1, 2).
   Para cada uno, un pequeño programa *que escribiste tú* dice dónde cae esa esquina en la
   pantalla, y quizá le pega un color. Este programa es el **vertex shader**. Se ejecuta
   una vez por esquina: tres veces para un triángulo.
2. **Puesto 2 — "¿qué píxeles quedan dentro?"** Una máquina (no tu programa) une las
   esquinas en un triángulo y lista cada píxel que cubre. Para un píxel entre las esquinas,
   *mezcla* lo que llevaran las esquinas: a medio camino entre una esquina roja y una azul
   sale morado. Esto es la **rasterización**, y la mezcla es la **interpolación**.
3. **Puesto 3 — "¿de qué color es este píxel?"** Para cada píxel cubierto, otro pequeño
   programa *que escribiste tú* decide el color final. Es el **fragment shader**. Se
   ejecuta una vez por píxel cubierto — miles de veces para un triángulo grande.
4. **Puesto 4 — "escríbelo en el dibujo"**, con interruptores como "qué canales de color
   se pueden escribir" y "¿se mezcla con lo que ya hay?".

Aquí viene el giro de Vulkan: no configuras la cadena puesto a puesto mientras funciona.
Describes **la cadena entera, todos los interruptores, de una vez**, y Vulkan la construye
como un solo objeto congelado — el **pipeline gráfico**. Por qué congelado: el driver
puede mirar toda la cadena a la vez y compilarla en la forma más eficiente para esta GPU.
El precio: cambia cualquier interruptor y necesitas otro pipeline. Unos pocos interruptores
pueden dejarse "para ponerlos mientras se graba" — eso es el **dynamic state**.

Dos cosas más antes de los detalles. Tus programas de shader se escriben en GLSL (§ 2),
pero Vulkan solo acepta la forma compilada SPIR-V — así que un compilador, `glslc`, los
convierte en bytes al hacer el build, y **tu propio código** carga esos bytes en Vulkan. Y
para dibujar, ahora le dices a la GPU "empieza a dibujar en *esta* imagen" — con
**dynamic rendering**, que elegiste en vez de render passes (`DECISIONS.md`).

**Dónde deja de valer la analogía:** los puestos no procesan una esquina y luego un
píxel, en orden. La GPU ejecuta miles de copias de tus shaders a la vez, sin orden
garantizado entre ellas. Un shader no puede ver lo que hacen sus vecinos.

---

## Los nombres

**shader module** — `VkShaderModule`: el handle de Vulkan para un bloque de código SPIR-V.
Lo creas a partir de palabras SPIR-V, lo usas mientras construyes el pipeline, y puedes
destruirlo justo después — el pipeline guarda su propia copia compilada.

**entry point** — la función de un shader por la que empieza, como `main` en C. En GLSL
siempre es `void main()`; la nombras (`c"main"`) al construir el pipeline.

**build script** — un archivo llamado `build.rs` junto al `Cargo.toml` de un crate. Cargo
lo compila y lo ejecuta **antes** de compilar el crate. El nuestro ejecuta `glslc` sobre
cada shader. `cargo:rerun-if-changed=shaders` (impreso por el script) le dice a Cargo que
lo vuelva a ejecutar cada vez que cambie esa carpeta.

**`glslc`** — **GLSL c**ompiler, del proyecto `shaderc` de Google (instalado por Homebrew
como `shaderc`). `glslc triangle.vert -o triangle.vert.spv`. Una herramienta que ejecutas,
no una librería que scop enlaza.

**`include_bytes!`** — una macro de Rust (el `!` marca una macro: código que escribe código
en tiempo de compilación) que copia los bytes de un archivo dentro del programa como
`&'static [u8]`. Junto con `env!("OUT_DIR")` — la carpeta donde Cargo les dice a los build
scripts que pongan los archivos generados — incrusta los shaders compilados en el binario.
No hay archivos de shader que buscar en tiempo de ejecución.

**SPIR-V magic number** — todo archivo SPIR-V empieza por la palabra de 32 bits
`0x07230203`. Comprobarlo es la primera línea de defensa contra cargar el archivo
equivocado. SPIR-V se define como una lista de *palabras* de 32 bits, así que el tamaño de
un archivo válido es múltiplo de 4.

**graphics pipeline** — `VkPipeline` creado con `create_graphics_pipelines`: la cadena de
producción congelada — shaders más todos los interruptores de función fija.

**fixed-function state** — los interruptores de los puestos que no programas: cómo las
esquinas se convierten en formas, cómo las formas se convierten en píxeles, cómo se
escriben los píxeles. "Función fija" porque antes eran hardware que hacía un único trabajo
fijo.

**input assembly** — cómo se agrupa el flujo de esquinas en formas. *Topología*
`TRIANGLE_LIST`: cada 3 esquinas forman un triángulo separado.

**viewport** — sobre qué rectángulo de la imagen se estira la caja −1..+1 de NDC (§ 2).
Normalmente la imagen entera: x 0, y 0, ancho × alto del extent de la swapchain.

**scissor** — un rectángulo fuera del cual los píxeles se tiran. Normalmente también la
imagen entera. (El viewport *aplasta* el dibujo en un rectángulo; el scissor lo *recorta*.)

**dynamic state** — interruptores que se dejan fuera del pipeline congelado a propósito y
se ponen mientras se graba. scop hace **dinámicos el viewport y el scissor**, para que el
mismo pipeline siga funcionando cuando cambie el tamaño de la ventana (§ 16).

**cull mode** — si se tiran los triángulos que miran hacia el otro lado (*back-face
culling*). Ahorra trabajo en modelos 3D cerrados. Para un único triángulo plano: `NONE`.

**front face** — qué orden de esquinas cuenta como "mirando hacia ti": `COUNTER_CLOCKWISE`
(antihorario) o `CLOCKWISE` (horario), visto en pantalla. Solo importa con el culling
activado — y es la razón clásica de que un modelo sea invisible: culling activado, giro al
revés.

**colour write mask** — qué canales de color (R, G, B, A) puede escribir el pipeline. En el
struct de create-info de Vulkan vale *ninguno* por defecto: olvídala y todo se dibuja pero
nada se escribe. `RGBA` = escribir los cuatro.

**pipeline layout** — `VkPipelineLayout`: la lista de *entradas extra* que los shaders
pueden leer además de los vértices (uniforms, texturas — § 10, § 14). Vacía por ahora, pero
un pipeline tiene que tener una.

**dynamic rendering** — empezar una pasada de dibujo con `cmd_begin_rendering`, nombrando
ahí mismo en el command buffer las image views en las que se dibuja. Sin objetos render
pass ni framebuffer.

**`VK_KHR_dynamic_rendering`** — la extensión de dispositivo que lo proporciona en Vulkan
1.2 (MoltenVK). En Vulkan 1.3 viene incluido. Activas la extensión **y** enciendes su
*feature* (`dynamic_rendering = true`) al crear el dispositivo — las extensiones añaden
funciones; las features son interruptores que también hay que encender.

**load op / store op** — qué pasa con el contenido de la imagen al empezar y al terminar de
dibujar. `LOAD_OP_CLEAR` = llenar antes con un color de limpieza (sustituye al
`cmd_clear_color_image` de § 5); `STORE_OP_STORE` = conservar el resultado (quieres
mostrarlo). Las alternativas `LOAD`/`DONT_CARE` importan más adelante para el rendimiento.

**`gl_VertexIndex`** — una entrada integrada de GLSL: *para qué esquina* es esta ejecución
del vertex shader (0, 1, 2…). Te permite dibujar un triángulo sin ningún dato de vértices —
las posiciones viven en el shader. (`gl_` = integrado, heredado de OpenGL.)

**`gl_Position`** — la salida integrada que el vertex shader debe escribir: dónde cae la
esquina, como un `vec4` (x, y, z, w). Por ahora `w = 1.0` y `z = 0.0`; § 9–10 explican
para qué sirven.

**location** — `layout(location = 0)`: un hueco numerado que conecta una salida del vertex
shader con una entrada del fragment shader. Mismo número en los dos lados = mismo valor.
Los nombres no importan; los números sí.

**interpolation** — mezclar valores por esquina a lo largo del triángulo para cada píxel,
ponderados según lo cerca que esté el píxel de cada esquina. Lo hace el rasterizador, para
cada `out` que escribe el vertex shader.

---

## Cómo funciona realmente

### Las piezas y dónde viven

```
scop/shaders/triangle.vert ─┐   build.rs ejecuta glslc  ┌─ OUT_DIR/triangle.vert.spv
scop/shaders/triangle.frag ─┴──────────────────────────►└─ OUT_DIR/triangle.frag.spv
                                                                 │ include_bytes!
                                                                 ▼
scop (main.rs)  ── bytes ──►  engine: bytes → palabras SPIR-V → shader modules
                                         → pipeline layout + pipeline gráfico
                                         → draw_frame: begin_rendering, bind, draw(3)
```

Los shaders pertenecen a la app (scop decide qué dibujar); construir pipelines a partir de
ellos pertenece al engine (todo proyecto lo necesita).

### 1. Los shaders

```glsl
// triangle.vert
#version 450                                   // versión de GLSL que debe usar el compilador
layout(location = 0) out vec3 colour;          // hueco 0 → el fragment shader

vec2 positions[3] = vec2[](vec2(0.0, -0.5), vec2(0.5, 0.5), vec2(-0.5, 0.5));
vec3 colours[3]   = vec3[](vec3(1, 0, 0),    vec3(0, 1, 0),  vec3(0, 0, 1));

void main() {
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
    colour = colours[gl_VertexIndex];
}
```

```glsl
// triangle.frag
#version 450
layout(location = 0) in vec3 colour;           // hueco 0 ← interpolado desde las esquinas
layout(location = 0) out vec4 out_colour;      // hueco 0 → colour attachment 0

void main() {
    out_colour = vec4(colour, 1.0);
}
```

`vec2/vec3/vec4` = vectores de 2/3/4 floats. Recuerda que **la y de Vulkan apunta hacia
abajo**: y = −0.5 es la esquina de *arriba*.

### 2. Compilarlos: `scop/build.rs`

Para cada archivo `.vert`/`.frag` en `scop/shaders/`: ejecuta `glslc <archivo> -o
$OUT_DIR/<archivo>.spv`; si `glslc` falta o falla, para el build con su mensaje de error.
Imprime `cargo:rerun-if-changed=shaders`.

### 3. Cargarlos: nuestro propio código

`include_bytes!` da bytes; Vulkan quiere palabras de 32 bits. El cargador:
1. comprueba que la longitud es un múltiplo de 4 distinto de cero;
2. convierte cada grupo de 4 bytes en un `u32`, **little-endian** (byte más bajo primero —
   así se guarda SPIR-V y así lo escribe `glslc` en cualquier máquina que vayas a usar);
3. comprueba que la primera palabra es el magic number `0x07230203`.
Luego `create_shader_module` con esas palabras. (Un cast directo de `&[u8]` → `&[u32]`
sería comportamiento indefinido: no está garantizado que los bytes empiecen en una
dirección alineada a 4 bytes.)

### 4. Activar dynamic rendering

- **Al elegir la GPU:** exige también `VK_KHR_dynamic_rendering`, y comprueba que la
  feature está soportada (`get_physical_device_features2` con un
  `PhysicalDeviceDynamicRenderingFeatures` encadenado — la idea de `push_next` de § 3).
- **Al crear el dispositivo:** activa la extensión, y encadena
  `PhysicalDeviceDynamicRenderingFeatures { dynamic_rendering: true }` al create info del
  dispositivo.
- **Al llamarlo:** en un dispositivo Vulkan 1.2 las funciones vienen de la propia tabla de
  funciones de la extensión, `ash::khr::dynamic_rendering::Device` (como con la swapchain
  en § 4) — no directamente de `Device`.

### 5. Construir el pipeline

Un `GraphicsPipelineCreateInfo`, rellenado con estas partes:

| Parte | Ajuste |
|---|---|
| Etapas de shader | módulo de vértices + módulo de fragmentos, entrada `main` |
| Vertex input | vacío (las posiciones están en el shader) |
| Input assembly | `TRIANGLE_LIST` |
| Viewport / scissor | 1 de cada, valores **dinámicos** |
| Rasterización | `FILL`, cull `NONE`, front face `COUNTER_CLOCKWISE`, ancho de línea 1.0 |
| Multisample | 1 muestra (sin antialiasing) |
| Colour blend | blending apagado, write mask `RGBA` |
| Estados dinámicos | `VIEWPORT`, `SCISSOR` |
| Layout | el pipeline layout vacío |
| Rendering info (encadenado) | un colour attachment, formato = **el formato de la swapchain** |

La última fila es lo que sustituye al render pass en dynamic rendering: el pipeline solo
necesita saber los *formatos* en los que va a dibujar.

### 6. Grabar un fotograma ahora

```
barrera: UNDEFINED → COLOR_ATTACHMENT_OPTIMAL
         (dst stage COLOR_ATTACHMENT_OUTPUT, dst access COLOR_ATTACHMENT_WRITE)
begin_rendering: colour attachment = la VIEW de esta imagen, layout COLOR_ATTACHMENT_OPTIMAL,
                 load CLEAR (color de limpieza), store STORE, área = el extent entero
bind_pipeline(GRAPHICS, pipeline)
set_viewport, set_scissor        (el extent entero)
draw(3 vértices, 1 instancia, primer vértice 0, primera instancia 0)
end_rendering
barrera: COLOR_ATTACHMENT_OPTIMAL → PRESENT_SRC_KHR
         (src stage COLOR_ATTACHMENT_OUTPUT, src access COLOR_ATTACHMENT_WRITE)
```

Y ahora el submit espera a `image_free` en `COLOR_ATTACHMENT_OUTPUT` — la primera etapa que
escribe en la imagen. La swapchain ya no necesita `TRANSFER_DST`.

*Instancia* en `draw(3, 1, …)`: dibujar la misma forma varias veces en una sola llamada.
Aquí, una copia.

### 7. Ownership

El pipeline y su layout se crean a partir del dispositivo, así que se destruyen antes que
él (y después de `device_wait_idle`). Los shader modules se pueden destruir justo después
de crear el pipeline.

---

## Trampas conocidas

- **Ventana vacía, sin errores** — las causas más comunes, por orden: colour write mask
  vacía; culling activado con las esquinas en el orden equivocado; el triángulo está fuera
  de pantalla (posiciones fuera de −1..+1); olvidaste `cmd_draw`.
- **Triángulo boca abajo** comparado con un tutorial — el tutorial es de OpenGL (y hacia
  arriba). La y de Vulkan apunta hacia abajo.
- **Error de validación "dynamic state not set"** (p. ej. *vkCmdDraw: …
  VK_DYNAMIC_STATE_VIEWPORT … not set*) — hiciste dinámicos el viewport/scissor pero no
  llamaste a `cmd_set_viewport` / `cmd_set_scissor` en este command buffer.
- **Error de validación de formato que no coincide** en `begin_rendering` o en el draw — el
  formato de color del pipeline no es el de la swapchain.
- **`cmd_begin_rendering` revienta o "function not loaded"** — la llamaste a través de
  `Device` en un dispositivo 1.2. Usa la tabla de funciones de la extensión.
- **Error de validación de que la feature `dynamicRendering` no está activada** —
  activaste la extensión pero no la feature.
- **El build falla: "glslc: not found"** — `glslc` no está instalado o no está en el `PATH`
  (`brew install shaderc`).
- **Los cambios en los shaders no aparecen** — el build script no se volvió a ejecutar.
  Comprueba que imprime `cargo:rerun-if-changed=shaders`.
- **`invalid magic number` desde tu cargador** — incrustaste el texto GLSL, no la salida
  `.spv`.
- **Entrada del fragment shader nunca asignada / colores basura** — los números de
  `location` no coinciden entre los dos shaders.

---

## Checkpoint

**Cumple:** "renderizarlo en una ventana" — la primera forma real en pantalla.
(Método: primero el gate, luego Claude lo escribe y te lo explica.)

1. `scop/shaders/triangle.vert` y `triangle.frag`; `scop/build.rs` compilándolos con
   `glslc`; shaders incrustados con `include_bytes!`.
2. Engine: nuestro propio cargador de SPIR-V (longitud, palabras little-endian, magic
   number) + tests unitarios para él.
3. Engine: dynamic rendering comprobado al elegir la GPU, activado (extensión + feature)
   al crear el dispositivo.
4. Engine: un pipeline construido a partir de los bytes de shader que pasa la app;
   destruido en `Drop`.
5. `draw_frame` pasa de `cmd_clear_color_image` a dynamic rendering: limpiar con la load
   op, dibujar 3 vértices. La swapchain deja de tener `TRANSFER_DST`.
6. **Demuestra que la write mask importa:** déjala vacía, ejecuta, mira el fondo sin
   triángulo, vuelve a ponerla.

**Verificar:** un triángulo RGB (rojo arriba, verde abajo a la derecha, azul abajo a la
izquierda) sobre el fondo que pulsa; ningún mensaje de validación; `cargo test` pasa los
tests del cargador; `cargo clippy` limpio.

---

## Preguntas

Q-6.1 (recuerdo) Para un triángulo que cubre 1000 píxeles: ¿cuántas veces se ejecuta el
vertex shader, cuántas el fragment shader, y qué decide cada uno?

Q-6.2 (predicción) Las tres esquinas son roja, verde y azul. ¿De qué color es un píxel justo
en el centro del triángulo, y qué parte de la cadena produjo ese color — tu código o la
GPU?

Q-6.3 (predicción) Copias las posiciones de las esquinas de un tutorial de OpenGL: esquina
superior en y = +0.5. ¿Dónde aparece esa esquina en tu ventana?

Q-6.4 (aplicación) Nombra dos ajustes del pipeline (o de la grabación) que deban coincidir
con la swapchain, de dónde viene cada valor, y qué verías si uno estuviera mal.

Q-6.5 (predicción) Cambias `triangle.frag` para que devuelva rojo puro y ejecutas
`cargo run` sin tocar ningún archivo de Rust. ¿Se vuelve rojo el triángulo? Explica qué
pasa entre guardar el archivo y que se abra la ventana.

Respuestas: `ANSWERS.md` § 6
