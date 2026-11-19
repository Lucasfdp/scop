---
section: 3
slug: vulkan-instance-and-device
tier: T2
status: done
prereqs: [2]
terms: [Vulkan loader, ICD, VkInstance, extension, layer, validation layer, debug messenger, VK_KHR_portability_enumeration, VK_KHR_portability_subset, physical device, queue family, logical device, handle, CreateInfo, VkResult, ash]
---

# Sección 3 — Hablar con la GPU

**Profundidad: T2** — todo objeto de Vulkan posterior se crea *a partir del* dispositivo
que creas aquí, y cada bug de Vulkan que tengas en tu vida lo reportará la validation
layer que activas aquí. Hazlo bien una vez y se reutiliza casi sin cambios en todos los
proyectos futuros.
**Serás capaz de:** arrancar Vulkan en tu Mac, elegir la GPU, abrir una sesión con ella,
obtener una cola a la que enviar trabajo y ver los propios informes de error de Vulkan en
tu terminal.

---

## La idea

Antes de poder dibujar nada necesitas tres presentaciones, un poco como llamar por
teléfono a una empresa:

1. **Pasar por la centralita.** Tu programa llama a una pequeña librería instalada en el
   sistema. No dibuja nada por sí misma; su único trabajo es encontrar el driver real de
   tu GPU y pasarle tus llamadas. Le dices quién eres y qué funciones opcionales te
   gustaría tener.
2. **Elegir con quién hablar.** La centralita lista las GPUs de la máquina. Un PC gaming
   puede tener dos (una integrada en el procesador y una tarjeta gráfica). Miras qué
   puede hacer cada una y eliges. En tu Mac hay exactamente una.
3. **Abrir una línea con esa GPU.** Abres una sesión con la GPU elegida, dices qué
   funciones extra necesitas y recibes una "bandeja de entrada" — el sitio donde
   dejarás tus listas de la compra de trabajo (los command buffers de § 2).

Vulkan también te da un **inspector** opcional: una capa de comprobación que se sitúa
entre tu programa y el driver y lee cada llamada que haces. Si pasas un valor erróneo,
olvidas destruir algo o rompes una regla de la especificación, imprime un mensaje
detallado. Sin él, un error a menudo no hace *nada visible* — o revienta tres llamadas
después en un sitio que no tiene nada que ver. **Desarrollarás con él activado,
siempre.**

**Dónde deja de valer la analogía del teléfono:** una llamada tiene una sola línea. En
Vulkan la GPU ofrece varios *tipos* de bandeja (para dibujar, para cálculo puro, para
copiar datos), y tienes que pedir una que pueda dibujar.

---

## Los nombres

**Vulkan loader** — la centralita: `libvulkan.dylib` en macOS (`libvulkan.so` en
Linux). Encuentra los drivers instalados y reenvía tus llamadas. Lo instaló
`vulkan-loader` de Homebrew.

**ICD** — **I**nstallable **C**lient **D**river. Un driver de Vulkan que el loader
encuentra mediante un pequeño archivo JSON (`MoltenVK_icd.json` en tu Mac).
"Installable" porque se pueden instalar varios a la vez; "client" porque se ejecuta
dentro del proceso de tu programa, a diferencia de la parte del driver que vive en el
kernel.

**`VkInstance`** — tu conexión con el loader: el paso 1. `Vk` es el prefijo de todo tipo
de Vulkan (`vk` en funciones, `VK_` en constantes). Se crea la primera y se destruye la
última.

**extension** — una función opcional que no está en el núcleo de Vulkan. Hay
**extensiones de instancia** (se piden al crear la instancia, p. ej. "voy a necesitar
dibujar en una ventana") y **extensiones de dispositivo** (se piden al crear el
dispositivo lógico, p. ej. "quiero una swapchain"). Los nombres siguen
`VK_<quién>_<qué>`: `KHR` = Khronos, las oficiales; `EXT` = de varios fabricantes;
etiquetas como `NV`, `AMD` para un solo fabricante.

**layer** — código que el loader inserta *entre* tu programa y el driver. Toda llamada
pasa por él. Se usa para comprobar, registrar y depurar.

**validation layer** — el inspector: `VK_LAYER_KHRONOS_validation`. Comprueba cada llamada
contra la especificación e informa de las infracciones. La instaló
`vulkan-validationlayers` de Homebrew. Cuesta rendimiento, así que es solo para builds
de desarrollo.

**debug messenger** — el objeto que recibe los mensajes de la validation layer y llama a
una función que le das tú (para que puedas imprimirlos). Viene de la extensión de
instancia `VK_EXT_debug_utils` ("utilidades de depuración").

**`VK_KHR_portability_enumeration`** — **enumerar** (listar) drivers de **portabilidad**.
MoltenVK es una implementación de *portabilidad*: implementa casi todo Vulkan, pero no el
100 %, porque Metal por debajo no puede hacerlo todo. El loader oculta esos drivers a
menos que actives esta extensión de instancia **y** pongas el flag
`ENUMERATE_PORTABILITY_KHR` en la instancia. Olvida cualquiera de los dos y Vulkan no
encuentra ninguna GPU. La mayoría de tutoriales (escritos para Windows/Linux) no lo
mencionan.

**`VK_KHR_portability_subset`** — la pareja en el lado del dispositivo: "sé que este
driver solo implementa un subconjunto". Si un dispositivo la lista como disponible,
**estás obligado** a activarla al crear el dispositivo lógico.

**physical device** — `VkPhysicalDevice`: una GPU real, tal como la reporta la instancia.
No la creas; las listas y eliges. Información de solo lectura: nombre, tipo, límites,
funciones soportadas.

**queue family** — un grupo de colas de una GPU con las mismas capacidades, identificado
por un índice (0, 1, 2…). Cada familia tiene flags: `GRAPHICS` (puede dibujar),
`COMPUTE`, `TRANSFER` (puede copiar). Necesitas el índice de una con `GRAPHICS`.

**logical device** — `VkDevice`: tu sesión con la GPU elegida (paso 3). Casi todo objeto
posterior se crea a partir de él y debe destruirse antes que él. "Lógico" porque es tu
vista en software del físico; varios programas pueden tener cada uno su propio
dispositivo lógico sobre la misma GPU.

**handle** — lo que Vulkan te da por cada objeto: un número opaco, como un descriptor de
archivo en C. `VkDevice`, `VkQueue`, `VkBuffer`… son todos handles. Copiar un handle no
copia el objeto — exactamente igual que copiar un `int fd`.

**CreateInfo** — el patrón de struct para crear cualquier cosa: `vkCreateInstance` recibe
un `VkInstanceCreateInfo`, `vkCreateDevice` recibe un `VkDeviceCreateInfo`, y así
sucesivamente. Rellenas un struct que describe lo que quieres y pasas un puntero a él.
Cada uno empieza con `sType` (structure type — qué struct es este) y `pNext` (pointer to
next — una cadena para structs de extensiones). `ash` rellena `sType` por ti.

**`VkResult`** — el código de retorno de casi toda función de Vulkan: `VK_SUCCESS`, o un
error como `VK_ERROR_INCOMPATIBLE_DRIVER`. En `ash` llega como un
`Result<T, vk::Result>` de Rust, así que `?` funciona.

**`ash`** — el crate de Rust que usas para Vulkan. No es un acrónimo: es un juego de
palabras — lo que queda tras el fuego de Vulkan (ceniza). Un binding fino 1:1:
`ash::Entry` guarda el loader cargado; `ash::Instance` y `ash::Device` guardan los
punteros a función de esa instancia y ese dispositivo. Toda llamada a Vulkan a través de
`ash` es `unsafe`.

---

## Cómo funciona realmente

### El orden de las operaciones

```
ash::Entry::load()                 abre libvulkan.dylib (el loader)
      │
      ▼
crear instancia  ◄── info de la app, extensiones de instancia, layers, flag de portabilidad
      │
      ├── crear debug messenger    (builds de desarrollo)
      │
      ▼
enumerar physical devices ──► revisar cada uno: nombre, queue families, extensiones ──► elegir uno
      │
      ▼
crear logical device  ◄── qué queue family, cuántas colas, extensiones de dispositivo
      │
      ▼
obtener handle de la cola  (índice de familia, cola 0)
```

La destrucción es el espejo exacto: **device → debug messenger → instance**. Las colas y
los physical devices no se destruyen — pertenecen a sus padres.

### 1. Cargar el loader

`ash::Entry::load()` abre `libvulkan.dylib` en tiempo de ejecución. En macOS con Homebrew
la librería vive en `/opt/homebrew/lib`. Si `Entry::load()` falla con "library not
found", esa carpeta no está en la ruta de búsqueda — el arreglo es poner
`DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib` (más adelante, en la regla de ejecución
del Makefile). *No estoy seguro de que tu configuración lo necesite — que `vkcube`
funcione es buena señal — así que pruébalo.*

### 2. Crear la instancia

Rellenas, de dentro hacia fuera:

- **application info** — el nombre y la versión de tu app, el nombre del motor y la
  **versión de API** a la que apuntas. Usa Vulkan 1.2 o 1.3; mira qué reporta
  `vulkaninfo --summary` para MoltenVK y no pidas más.
- **extensiones de instancia** — por ahora `VK_KHR_portability_enumeration` y
  `VK_EXT_debug_utils`. § 4 añade las de la ventana.
- **layers** — `VK_LAYER_KHRONOS_validation`, solo en builds de depuración
  (`cfg!(debug_assertions)` es true con `cargo build`, false con `--release`).
- **flags** — `ENUMERATE_PORTABILITY_KHR`.

Los nombres de extensiones y layers son strings de C. Rust 2021+ tiene literales de
string de C (`c"VK_LAYER_KHRONOS_validation"`), y `ash` da los nombres de extensión como
constantes (`ash::khr::portability_enumeration::NAME`, `ash::ext::debug_utils::NAME`). Le
pasas a Vulkan un *array de punteros* a ellos (`.as_ptr()` en cada uno) — la regla de § 1
de "el dueño debe vivir más que el puntero", en la práctica. Los literales y las
constantes viven todo el programa, así que son seguros.

### 3. El debug messenger

Le das una `extern "system" fn` — una función con la convención de llamada de C, para que
el driver en C pueda llamarla. Recibe una severidad (verbose, info, warning, error), un
tipo y el mensaje. Imprime warnings y errores; al principio sáltate verbose e info,
porque MoltenVK es muy hablador. Devuelve `vk::FALSE` ("no abortes la llamada").

### 4. Elegir el physical device

Para cada physical device:
- lee sus propiedades (nombre, tipo, versión de API);
- lista sus queue families y busca el índice de una con `GRAPHICS`;
- (§ 4 añade: ¿puede presentar en la ventana, y soporta la extensión de swapchain?)

Quédate con el primero que pase. En tu Mac: "Apple M-algo".

### 5. Crear el logical device

Dices: de la queue family *N* quiero *1* cola, prioridad `1.0` (las prioridades solo
importan con varias colas). Más extensiones de dispositivo: por ahora
`VK_KHR_portability_subset` **si el dispositivo la lista** (compruébalo con
`enumerate_device_extension_properties`). § 4 añade `VK_KHR_swapchain`.

Después `get_device_queue(family_index, 0)` te entrega la cola.

### 6. Poseerlo todo en Rust

Un struct — un contexto de GPU — guarda el entry, la instancia, el debug messenger, el
physical device, el logical device, la cola y el índice de familia. Implementa `Drop` y
destruye explícitamente, en orden espejo. No te fíes del orden de los campos (la trampa
de § 1): lo explícito es más fácil de leer y de defender.

---

## Trampas conocidas

- **`ERROR_INCOMPATIBLE_DRIVER` al crear la instancia**, o "0 physical devices". Te
  olvidaste de `VK_KHR_portability_enumeration` o del flag `ENUMERATE_PORTABILITY_KHR`.
  Necesitas los dos.
- **`ERROR_LAYER_NOT_PRESENT`** — la validation layer no está instalada o no se
  encuentra. Comprueba que `vulkaninfo --summary` la lista. Si falta, arranca sin ella
  (con un aviso) en vez de negarte a arrancar — puede que la máquina de evaluación no la
  tenga.
- **La validación parece "callada"** — la layer no está realmente activada (build
  release, o no se encontró y saltó tu alternativa), o tu callback filtra esa severidad.
  (Sin ningún messenger, la layer suele escribir en stdout por su cuenta — pero eso no
  lo puedes filtrar ni formatear.) Pruébalo: sáltate una vez la destrucción del
  dispositivo; tienes que ver un error al salir.
- **Error de validación sobre `VK_KHR_portability_subset`** — el dispositivo la lista y
  no la activaste.
- **Nombres corruptos o crashes aleatorios al crear la instancia** — un puntero colgante
  a un `CString` temporal (§ 1). Los literales `c"..."` y las constantes `NAME` de `ash`
  lo evitan.
- **"objects not destroyed" al salir** — orden de destrucción incorrecto o un destroy que
  falta.
- **Pedir una versión de API más nueva de la que soporta MoltenVK** — puede que la
  creación funcione igualmente, pero luego fallan las funciones nuevas. Ajústate a
  `vulkaninfo`.

---

## Checkpoint

**Cumple:** nada visible todavía — esta es la base sobre la que se apoya todo requisito de
renderizado. (Método: primero el gate, luego Claude lo escribe y te lo explica.)

1. `ash` añadido al engine, con versión fijada (`ash = "=0.38.0"`).
2. Un tipo de contexto de GPU en el engine que, al crearse: carga el loader, crea la
   instancia (portabilidad + debug utils, validación en builds de depuración), crea el
   debug messenger, elige un physical device con una queue family gráfica, crea el
   logical device y obtiene la cola. Al hacer drop: destruye en orden espejo.
3. Imprime el nombre de la GPU, su versión de API y el índice de la queue family elegida.
4. Se crea desde scop **antes** de que se abra la ventana, por ahora (la ventana se une
   en § 4).
5. **Demuestra que la validación funciona:** comenta la destrucción del dispositivo,
   ejecuta, mira el error, vuelve a ponerla.

**Verificar:** nombre de la GPU impreso; ningún mensaje de validación en una ejecución
normal; un error cuando te saltas un destroy; `cargo clippy` limpio.

---

## Preguntas

Q-3.1 (recuerdo) Nombra las tres cosas que creas en esta sección, en orden de creación, y
el orden en que las destruyes.

Q-3.2 (predicción) Sigues al pie de la letra un tutorial de Vulkan para Windows en tu Mac.
La creación de la instancia falla, o se encuentran cero GPUs. ¿Cuál es la causa más
probable, y qué dos cosas lo arreglan?

Q-3.3 (aplicación) Una GPU reporta tres queue families: la familia 0 tiene
`COMPUTE | TRANSFER`, la familia 1 tiene `GRAPHICS | COMPUTE | TRANSFER`, la familia 2
tiene `TRANSFER`. ¿Qué índice usas, y por qué?

Q-3.4 (predicción) Tu código activa la validación solo cuando `cfg!(debug_assertions)` es
true. Ejecutas `cargo run --release` y olvidas destruir un buffer. ¿Qué ves en la
terminal, y por qué?

Q-3.5 (predicción) Copias un handle `VkDevice` en dos variables y llamas a
`destroy_device` a través de cada una. ¿Qué pasa, y cómo puede el ownership de Rust
impedir que esto ocurra en el engine?

Respuestas: `ANSWERS.md` § 3
