---
section: 10
slug: uniforms-and-descriptors
tier: T2
status: done
prereqs: [8, 9]
terms: [uniform, per-frame vs per-draw data, push constant, push constant range, stage flags, pipeline layout, uniform buffer, persistently mapped, descriptor, descriptor set layout, descriptor pool, descriptor set, set and binding numbers, descriptor write, binding a descriptor set, std140, view-projection matrix]
---

# Sección 10 — Pasar datos por fotograma a los shaders

**Profundidad: T2** — así es como *cualquier* dato de fuera llega a un shader a partir de
ahora (cámara, modelo, texturas en § 14, el valor del fundido en § 15, luces en ft_vox).
Deberías saber decir dónde vive cada valor, por qué, y qué se rompe si está en el sitio
equivocado.
**Serás capaz de:** enviar las matrices de § 9 al vertex shader y ver un cubo girando en
perspectiva.

---

## La idea

Hasta ahora el vertex shader solo tenía entradas **por esquina**: la posición y el color
propios de cada esquina, leídos del vertex buffer (§ 7). Pero la matriz de § 9 es distinta:
es la **misma para todas las esquinas** del cubo. Todas giran el mismo ángulo y se alejan
la misma distancia. El shader necesita una forma de recibir "ajustes para todo el draw",
además de "datos de esta esquina".

*Podrías* aplicar la matriz en la CPU y volver a subir todas las esquinas cada fotograma.
Para un cubo de 8 esquinas va bien. Para la tetera significa reescribir 3644 esquinas 60
veces por segundo, y todo el trabajo de la GPU es hacer esas cuentas en paralelo. Así que el
plan es: las esquinas se quedan intactas en la memoria de la GPU (subidas una vez, § 7), y
cada fotograma envías solo las matrices, unas pocas docenas de bytes.

Hay dos formas de darle esos ajustes a la GPU. Piensa en una cocina:

- **Un pósit pegado a la comanda.** Es diminuto y viaja *con* el pedido, así que cada
  pedido puede llevar una nota distinta gratis. En Vulkan la "comanda" es el command buffer
  (§ 5). La nota se copia dentro mientras grabas.
- **Un tablón de anuncios en la pared de la cocina.** Cabe mucho más, y muchos pedidos
  pueden leer el mismo tablón. Pero hay que decirle al cocinero *qué* tablón mirar, y si
  reescribes el tablón mientras un cocinero aún está leyendo en él el menú de anoche, lee
  una mezcla de los dos.

scop usa los dos. La **cámara** (cómo se ve el mundo) cambia una vez por fotograma y es la
misma para todos los objetos, así que va al tablón. La **matriz de modelo** (cómo se coloca
y gira *este* objeto) es distinta para cada objeto, así que va en un pósit.

**Dónde deja de valer la analogía:** un tablón de verdad se lee "ahora". La GPU lo lee
*después*, cuando ejecuta de verdad el fotograma, que puede ser cuando la CPU ya ha
empezado a preparar el siguiente (§ 5: dos fotogramas en vuelo). Por eso scop tiene **un
tablón por slot de fotograma**, no uno compartido. Más en "Cómo funciona realmente" § 3.

---

## Los nombres

**uniform** — una entrada de shader que tiene el *mismo valor para todas las esquinas* (y
todos los píxeles) de un draw. "Uniforme" como en "igual de principio a fin", lo opuesto a
las entradas por esquina del vertex buffer. En GLSL es la palabra clave `uniform`.

**per-frame vs per-draw data** — los datos *por fotograma* cambian como mucho una vez por
fotograma y los comparten todos los draws (la cámara). Los datos *por draw* pueden ser
distintos entre dos draws del mismo fotograma (la matriz de modelo de cada objeto).
Clasificar los valores en estos dos cubos decide adónde van.

**push constant** — el pósit. Un bloque pequeño de bytes escrito directamente en el command
buffer con `cmd_push_constants`. *Push* porque empujas los bytes dentro de la grabación;
*constant* porque el valor no cambia durante el draw que lo lee. Vulkan garantiza al menos
**128 bytes** en cualquier GPU (el límite se llama `maxPushConstantsSize`, "tamaño máximo de
push constants"). Son exactamente dos `mat4`.

**push constant range** — `VkPushConstantRange`: la declaración "mis shaders leerán bytes
de push constant del offset 0 al 64, en la etapa de vértices". Vive en el pipeline layout
(abajo).

**stage flags** — `VkShaderStageFlags`: qué etapas de shader pueden ver una entrada.
`VERTEX` = el vertex shader, `FRAGMENT` = el fragment shader. Declarar la etapa equivocada
es un error de validación.

**pipeline layout** — `VkPipelineLayout`. Tienes uno *vacío* desde § 6. Es la lista de
entradas de fuera del pipeline: "tantos tablones con esta forma, más este espacio de
pósit". Los shaders solo pueden leer entradas que declare el layout. Piénsalo como la firma
de función de los parámetros extra del pipeline.

**uniform buffer** — un `VkBuffer` normal (§ 7) creado con el flag de uso `UNIFORM_BUFFER`
("un shader leerá esto como datos uniform"). Es el tablón. OpenGL lo llama **UBO**,
"Uniform Buffer Object", y verás ese nombre por todas partes.

**persistently mapped** — *mapear* (§ 7) le da a la CPU un puntero a memoria visible para
la GPU. *Persistentemente* significa mapearla **una vez** al crearla y guardar el puntero
para siempre, en vez de map → escribir → unmap cada fotograma. Va bien con memoria
`HOST_COHERENT`, donde las escrituras se hacen visibles para la GPU sin una llamada de
flush extra.

**descriptor** — un registro diminuto que *describe* un recurso a un shader: "el buffer es
*este* `VkBuffer`, empezando en el byte 0, de 64 bytes". Es la nota que le dice al cocinero
qué tablón leer. No contiene los datos; **apunta a** ellos.

**descriptor set** — `VkDescriptorSet`: un grupo de descriptors que se enganchan juntos, en
una llamada. El de scop tiene un descriptor (el buffer de la cámara). § 14 añade la textura
a un set.

**descriptor set layout** — `VkDescriptorSetLayout`: la *forma* de un set sin los recursos
reales: "el binding 0 es un uniform buffer, visible para la etapa de vértices". Es como la
declaración de un struct; el descriptor set es una instancia de él.

**descriptor pool** — `VkDescriptorPool`: de donde se reservan los descriptor sets. La
misma idea que el command pool (§ 5): lo dimensionas por adelantado ("espacio para 2 sets
con 2 descriptors de uniform buffer en total"), y destruir el pool libera todos sus sets.

**set and binding numbers** — la dirección de un descriptor para el shader:
`layout(set = 0, binding = 0)` significa "hueco de descriptor set 0, entrada 0 dentro de
él". La misma idea que `location` para las entradas de vértices (§ 7), con dos niveles.

**descriptor write** — `VkWriteDescriptorSet` + `update_descriptor_sets`: rellenar un set,
es decir, hacer que "el binding 0 del set A" apunte a un buffer concreto. Se hace **una
vez**. Después cambias el *contenido del buffer*, nunca el descriptor.

**binding a descriptor set** — `cmd_bind_descriptor_sets`: un comando grabado en el command
buffer que dice "a partir de aquí, el hueco de set 0 es *este* set". Como
`cmd_bind_pipeline` (§ 6), sigue vigente para los draws siguientes.

**std140** — la regla que usa GLSL para disponer en memoria los bytes de un bloque uniform:
disposición **st**an**d**ard, introducida en la versión **1.40** de GLSL. Para un `mat4`
significa 16 floats, 64 bytes, **columna a columna**. Así se guarda exactamente el `Mat4` de
§ 9 (`#[repr(C)]`, `cols[c][r]`), así que los bytes se pueden copiar tal cual. Otros tipos
tienen trampas de relleno (un `vec3` ocupa 16 bytes, no 12); scop las evita enviando solo
`mat4`s.

**view-projection matrix** — `projection × view` multiplicadas en la CPU *una vez por
fotograma* en una sola tarjeta: "ver el mundo desde la cámara, luego aplastarlo sobre la
pantalla". Así el shader solo multiplica por dos matrices, no por tres.

---

## Cómo funciona realmente

### 1. El lado del shader

El vertex shader gana dos entradas y pasa a 3D:

```glsl
#version 450

// Tablón: set 0, binding 0. Lo escribe el engine una vez por fotograma.
layout(set = 0, binding = 0) uniform Camera {
    mat4 view_proj;   // projection × view
} camera;

// Pósit: se empuja antes de cada draw.
layout(push_constant) uniform Push {
    mat4 model;       // posición y giro de este objeto
} push;

layout(location = 0) in vec3 in_position;   // era vec2: ahora las esquinas son 3D
layout(location = 1) in vec3 in_colour;
layout(location = 0) out vec3 colour;

void main() {
    gl_Position = camera.view_proj * push.model * vec4(in_position, 1.0);
    colour = in_colour;
}
```

Lee la línea de `gl_Position` de derecha a izquierda, como en § 9: la esquina (un
**punto**, así que `w = 1.0`) la coloca la matriz de modelo, y luego se ve a través de la
cámara. `Camera` y `Push` son solo nombres de los bloques; `camera` y `push` son lo que usa
el código.

### 2. El pipeline layout se rellena

Hasta ahora: `create_pipeline_layout(&PipelineLayoutCreateInfo::default())`, una firma
vacía. Ahora declara las dos entradas:

```
descriptor set layout:  binding 0 → 1 × UNIFORM_BUFFER, etapa VERTEX
push constant range:    etapa VERTEX, offset 0, tamaño 64   (un mat4)
pipeline layout:        set 0 = ese set layout  +  ese rango de push
```

El engine construye **un** pipeline layout y todos los pipelines lo usan. Ventaja: cuando
dos pipelines comparten layout, un descriptor set enlazado una vez sigue siendo válido al
cambiar entre ellos, así que se enlaza una vez por fotograma, no una vez por draw.

Si un shader lee algo que el layout no declara (o declara la etapa o el tamaño
equivocados), la creación del pipeline da un error de validación. El layout y el GLSL deben
coincidir.

### 3. Un buffer de cámara por slot de fotograma

Recuerda § 5: hay `FRAMES_IN_FLIGHT = 2` slots de fotograma. Mientras la GPU sigue
dibujando el fotograma N desde el slot A, la CPU ya está grabando el fotograma N+1 en el
slot B. Si los dos slots compartieran un buffer de cámara, la CPU estaría reescribiendo la
matriz **mientras la GPU aún la está leyendo** para el fotograma anterior.

Así que cada slot posee su propio uniform buffer:

```
slot A: command buffer, fence, image_free … + buffer de cámara A + descriptor set A → buffer A
slot B: command buffer, fence, image_free … + buffer de cámara B + descriptor set B → buffer B
```

¿Cuándo es seguro escribir el buffer A? Después de esperar a la fence del slot A: ese es el
momento en que la GPU ha terminado todo lo enviado antes desde el slot A, y `draw_frame` ya
espera ahí (paso 1). Así que la escritura va **después de la espera de la fence**, antes de
grabar.

La memoria es `HOST_VISIBLE | HOST_COHERENT` y está mapeada persistentemente: la CPU
escribe los datos por fotograma cada fotograma, así que la ruta de staging de § 7 costaría
más de lo que ahorra. Esa es la nota de "revisar" de la decisión de § 7.

### 4. Descriptor pool y sets: se preparan una vez

Al arrancar, cuando ya existen los buffers:

1. Crea el descriptor set layout (la forma del paso 2).
2. Crea un descriptor pool con espacio para **2 sets** y **2 descriptors de uniform
   buffer**.
3. Reserva 2 sets de él, ambos con ese layout.
4. **Escribe** cada set: binding 0 del set A → buffer A (offset 0, 64 bytes); el del set B
   → buffer B.

Es la última vez que cambian los descriptors. Cada fotograma solo cambias los 64 bytes
*dentro* del buffer; el descriptor ya apunta ahí.

### 5. Cada fotograma

```
esperar fence del slot  →  acquire de imagen  →  reiniciar fence   (sin cambios, § 5)
copiar view_proj (64 bytes) al buffer mapeado de este slot          NUEVO
grabar:
  begin rendering, viewport, scissor                                (sin cambios)
  para cada draw:
    bind del pipeline                                               (sin cambios)
    bind del descriptor set: slot 0 = el set de este slot           NUEVO (barato; vale por draw)
    push constants: model, 64 bytes, etapa VERTEX                   NUEVO
    bind de vertex/index buffers, draw_indexed                      (sin cambios)
submit, present                                                     (sin cambios)
```

Enlazar el set dentro del bucle es la versión sencilla y siempre correcta. Enlazarlo una
vez antes del bucle también funciona porque todos los pipelines comparten el layout.
Cualquiera de las dos vale para scop.

### 6. Las esquinas pasan a 3D

`Vertex::position` pasa a `[f32; 3]`, el formato de su atributo pasa a `R32G32B32_SFLOAT`
("canales **R**ed **G**reen **B**lue, **32** bits cada uno, **S**igned **FLOAT**": los
nombres de colores son solo la forma de Vulkan de decir "3 floats"), y el shader lee
`vec3`. El quad antiguo recibe `z = 0.0`.

### 7. La API del engine

Lo que ve scop (solo handles, nada de Vulkan, como en § 8):

```rust
frame.aspect()                         // ancho ÷ alto de la imagen de este fotograma
frame.set_camera(projection * view)    // una vez por fotograma → el uniform buffer del slot
frame.draw(pipeline, mesh, model)      // por draw → push constant
```

`aspect()` se lee **cada fotograma** del tamaño de la swapchain, así que una ventana
redimensionada recibe la proyección correcta sin código extra. `Mat4` gana un pequeño método
para entregar sus 16 floats como bytes en orden de columnas. Su disposición ya coincide con
std140.

### 8. Qué hace scop con ello

```
update:  angle += speed × dt                    (segundos, así la velocidad es igual en cualquier máquina)
draw:    projection = perspective(60°→radianes, frame.aspect(), 0.1, 100.0)
         view       = translation(0, 0, −3)       // cámara 3 unidades por detrás del cubo
         frame.set_camera(projection × view)
         model      = rotation_y(angle) × rotation_x(angle × 0.5)
         frame.draw(pipeline, cube, model)
```

Las esquinas del cubo ya están centradas en el origen, así que no hace falta tarjeta de
centrado. El logo de 42 la necesitará (§ 13, Q-9.3).

### Decisión: push constants para el modelo, uniform buffer para la cámara

| Opción | A favor | En contra |
|---|---|---|
| **Las dos matrices como push constants** (2 × 64 = 128 bytes) | Menos objetos: sin buffers, sin descriptors | Usa *todo* el espacio garantizado: el valor del fundido de § 15 no cabría a su lado. Los descriptors habría que aprenderlos igualmente en § 14 para la textura, con un recurso más difícil |
| **Las dos en uniform buffers** | Un solo mecanismo | Datos por draw en buffers necesitan una región de buffer por objeto (u "offsets dinámicos"), más contabilidad para nada |
| **Cámara en un uniform buffer, modelo como push constant** ✅ | Cada valor está donde le corresponde (por fotograma vs por draw); aprende descriptors con el recurso más simple antes de § 14; deja 64 bytes de espacio de push; el patrón que usan los engines reales y ft_vox | Más objetos de Vulkan (set layout, pool, sets, 2 buffers) |

Recomendado: la tercera. Registrado en `DECISIONS.md`.

---

## Trampas conocidas

- **Error de validación al crear el pipeline que menciona el push constant range o un
  descriptor que no está en el layout** — el GLSL y el pipeline layout no coinciden: falta
  el rango de push, el tamaño no es 64, o el flag de etapa es el equivocado (`FRAGMENT` en
  vez de `VERTEX`).
- **Error de validación al dibujar: descriptor set no enlazado** — falta
  `cmd_bind_descriptor_sets` (o se grabó después del draw).
- **La cámara nunca se mueve aunque escribes una matriz nueva cada fotograma** — escribes
  en el buffer del slot equivocado, en un buffer que no es `HOST_COHERENT`, o a través de
  un puntero de un map que luego se deshizo.
- **Saltos o tirones ocasionales de un fotograma cuando la cámara se mueve rápido** — un
  buffer compartido entre slots, o escribir antes de la espera de la fence. La validation
  layer normalmente **no dice nada** de esto: es una carrera, no un mal uso de la API.
- **Cubo deformado, invertido o desaparecido sin error** — la matriz llega traspuesta
  (Q-9.5). El `mat4` de std140 es column-major y `Mat4` también; no añadas `row_major` en
  GLSL ni traspongas al subir.
- **Cubo estirado en una ventana ancha** — `aspect` calculado una vez en `setup`, o
  alto ÷ ancho en vez de ancho ÷ alto.
- **El cubo gira 57× demasiado rápido o como loco** — grados pasados a `perspective` o a
  una rotación.
- **Caras traseras dibujadas sobre las delanteras en algunos ángulos** — *esperado* en este
  checkpoint. La GPU dibuja los triángulos en orden de índice y todavía nada dice cuál está
  delante. Ese es todo el trabajo de § 11. Distintos colores por esquina lo hacen fácil de
  ver.
- **Error de validación al apagar: descriptor pool / set layout / buffer sin destruir** —
  los objetos nuevos hay que destruirlos después de `device_wait_idle`, antes del
  dispositivo. Destruir el pool libera sus sets; no se liberan uno a uno.

---

## Checkpoint

**Cumple (subject):** "perspectiva" y "gira sobre su eje de simetría principal" (con un
cubo por ahora; el logo de 42 llega en § 12–13). Método: primero el gate, luego Claude lo
escribe y te lo explica.

1. **Esquinas 3D:** `Vertex::position: [f32; 3]`, formato de atributo `R32G32B32_SFLOAT`,
   shader `in vec3`. Quad actualizado a `z = 0`.
2. **Engine:** un descriptor set layout + un push constant range en un pipeline layout
   compartido; un uniform buffer `HOST_VISIBLE | HOST_COHERENT` mapeado persistentemente
   por slot de fotograma; un descriptor pool y un set por slot, escritos una vez;
   `draw_frame` escribe la cámara tras la espera de la fence; `record_frame` enlaza el set
   y empuja la matriz de modelo de cada draw; todo lo nuevo destruido en el orden
   correcto.
3. **API:** `Frame::aspect()`, `Frame::set_camera(Mat4)`, `Frame::draw(pipeline, mesh,
   model)`; `Mat4` → floats en orden de columnas para subir. Ningún tipo de Vulkan se
   escapa a scop.
4. **scop:** un cubo (8 esquinas, 36 índices, un color distinto por esquina), girando con
   `dt`, en perspectiva, cámara 3 unidades por detrás.

**Verificar:**
- `cargo run -p scop`: el cubo gira suavemente; las aristas lejanas se ven más pequeñas que
  las cercanas.
- Redimensiona la ventana: `aspect` ya sigue el tamaño de la swapchain cada fotograma, pero
  la swapchain en sí solo se reconstruye en § 16 — hasta entonces una ventana
  redimensionada puede verse estirada o saltarse fotogramas. Es lo esperado aquí.
- Validation layer callada, también al salir (Esc).
- Algunas caras solapándose mal en ciertos ángulos: esperado, ver § 11.
- `cargo test` y `cargo clippy --all-targets` limpios.

---

## Preguntas

Q-10.1 (recuerdo) scop envía dos matrices al vertex shader de dos formas distintas. ¿Qué
matriz va dónde, y por qué le corresponde ese sitio a cada una?

Q-10.2 (predicción) Para ahorrar memoria, usas **un** uniform buffer de cámara para los dos
slots de fotograma en vez de uno cada uno. Nada más cambia. ¿Qué puede salir mal, por qué,
y te lo diría la validation layer?

Q-10.3 (aplicación) En `gl_Position = camera.view_proj * push.model * vec4(in_position, 1.0);`
¿qué significa el `1.0`, y en qué orden se aplican las transformaciones a la esquina?

Q-10.4 (predicción) El descriptor set de cada slot se escribe una vez al arrancar y nunca
más se toca, y aun así el shader ve una matriz de cámara nueva cada fotograma. ¿Cómo es
posible? ¿Qué *sí* necesitaría una nueva escritura de descriptor?

Q-10.5 (predicción) `aspect` se calcula una vez en `setup` y se guarda. El cubo gira
tranquilamente, y entonces estiras la ventana al doble de ancho. ¿Qué ves, y de dónde
debería salir `aspect` en su lugar?

Respuestas: `ANSWERS.md` § 10
