---
section: 4
slug: surface-and-swapchain
tier: T2
status: done
prereqs: [3]
terms: [surface, VK_KHR_surface, VK_EXT_metal_surface, raw-window-handle, ash-window, present, VK_KHR_swapchain, surface capabilities, surface format, sRGB, present mode, FIFO, MAILBOX, extent, VkImage, image view, ERROR_OUT_OF_DATE_KHR]
---

# Sección 4 — Llevar imágenes a la pantalla

**Profundidad: T2** — la swapchain es donde Vulkan se encuentra con la ventana, y es la
parte que se rompe cuando la ventana se redimensiona, se minimiza o pasa a una pantalla
Retina. Entender cada ajuste de aquí es lo que te permite arreglar esos bugs en segundos
en vez de en horas.
**Serás capaz de:** conectar Vulkan a tu ventana de `winit`, crear el conjunto de imágenes
que se turnan en pantalla y explicar cada decisión que tomaste al crearlas.

---

## La idea

Tras § 3 puedes hablar con la GPU, pero la GPU no tiene ni idea de que tu ventana existe.
Vulkan no sabe nada de ventanas a propósito — funciona en servidores sin pantalla, en
móviles, en consolas — así que necesitas un adaptador entre "una ventana en macOS" y "un
sitio donde Vulkan puede dejar imágenes terminadas".

Piensa en un teatro con un escenario (la ventana) y un pequeño juego de marcos idénticos
(las imágenes). Mientras el público mira el marco que está en escena, detrás del telón
los pintores pintan el siguiente. Cuando un cuadro está terminado se le entrega al regidor,
que lo cambia en escena en el momento justo — nunca a mitad de la vista del público — y
devuelve el marco anterior para pintarlo encima.

- El **adaptador** a la ventana es la *surface*.
- El juego de marcos, con las reglas para cambiarlos, es la *swapchain*.
- El regidor es el *motor de presentación* del sistema — el compositor de macOS, no tu
  código. Tú solo pides "dame un marco libre" y "por favor, pon este en escena".

Para crear el juego de marcos tienes que responder, por adelantado: ¿qué tamaño tienen los
marcos? ¿Qué formato de color? ¿Cuántos? ¿Y cuál es la regla de cambio — esperar al
refresco de la pantalla, o cambiar en cuanto un marco esté listo? Las respuestas no te las
inventas: le preguntas a la surface qué soporta y eliges de ese menú.

**Dónde deja de valer la analogía del teatro:** pintores y regidor trabajan a la vez, sin
gritarse. Coordinar "este marco está terminado" y "este marco vuelve a estar libre" se hace
con señales, que es § 5.

---

## Los nombres

**surface** — `VkSurfaceKHR`: el handle de Vulkan para "algo a lo que puedo presentar
imágenes", aquí tu ventana. Se crea a partir de la instancia; necesita que la ventana siga
viva mientras exista.

**`VK_KHR_surface`** — la extensión de instancia que define las surfaces en general.

**`VK_EXT_metal_surface`** — la extensión de instancia que crea una surface a partir de una
*capa Metal* de macOS (el área dibujable de una ventana, `CAMetalLayer`). Cada plataforma
tiene la suya: `VK_KHR_xlib_surface`, `VK_KHR_wayland_surface`, `VK_KHR_win32_surface`…
Por eso no las escribirás a mano en el código.

**`raw-window-handle`** — un crate diminuto que define un formato común para "aquí tienes
un handle de ventana, de alguna librería de ventanas, para alguna plataforma". `winit`
produce uno; el código de Vulkan consume uno. Nada más.

**`ash-window`** — un pequeño crate compañero de `ash` que lee un `raw-window-handle` y (a)
te dice qué extensiones de instancia de surface necesita esta plataforma, y (b) crea el
tipo de surface correcto. Solo pegamento de ventana — sin lógica de renderizado.
Registrado en `DECISIONS.md`.

**present** — entregar una imagen terminada al motor de presentación para que se muestre.
"Presentación" es la palabra de Vulkan para "ponerlo en pantalla".

**`VK_KHR_swapchain`** — la extensión de dispositivo que proporciona swapchains. La añades
al crear el logical device (paso 5 de § 3) — y compruebas que el physical device la
soporta.

**surface capabilities** — lo que permite la surface: número mínimo y máximo de imágenes,
tamaño actual, tamaños mínimo y máximo, transformaciones y modos de alpha soportados. Se
consulta; no se configura.

**surface format** — el formato de píxel (cuántos bits por canal de color, en qué orden)
más el espacio de color. Típicamente `B8G8R8A8_SRGB`: **B**lue (azul), **G**reen (verde),
**R**ed (rojo), **A**lpha, 8 bits cada uno, codificado en sRGB.

**sRGB** — **s**tandard **R**ed **G**reen **B**lue. Una codificación de color que dedica más
precisión a los tonos oscuros, porque el ojo humano es más sensible ahí. Con un formato
`_SRGB` la GPU convierte automáticamente al escribir: tu shader trabaja con valores
"lineales" (0.5 = la mitad de luz), la imagen los guarda codificados en sRGB. Importa para
tus "sutiles tonos de gris": el mismo número se ve distinto en las dos codificaciones.

**present mode** — la regla de cambio.

**FIFO** — **F**irst **I**n, **F**irst **O**ut (el primero que entra es el primero que sale):
las imágenes terminadas hacen cola y se muestra una por refresco de pantalla. Esto es
vsync: sin tearing, limitado a la frecuencia de refresco del monitor. **El único modo que
Vulkan garantiza en todas las plataformas.**

**MAILBOX** — como FIFO, pero la cola solo admite una imagen: una imagen terminada más
nueva *sustituye* a la que espera, como un buzón donde solo cabe la última carta. Menos
retardo, sin tearing, pero la GPU sigue trabajando a tope. Opcional — puede no ofrecerse.

**extent** — la palabra de Vulkan para un tamaño 2D en píxeles: `ancho × alto`.

**`VkImage`** — un bloque de memoria de la GPU que contiene píxeles, más una descripción
(tamaño, formato, uso). Las imágenes de la swapchain son `VkImage`s que **no posees** — la
swapchain las creó y la swapchain las destruye.

**image view** — `VkImageView`: describe *cómo mirar* una imagen — con qué formato leerla,
qué parte de ella (niveles de mip, capas). Los shaders y el renderizado nunca tocan un
`VkImage` directamente; pasan por una view. Creas una por imagen de la swapchain, y esas
*sí* las posees.

**`ERROR_OUT_OF_DATE_KHR`** — el resultado que obtienes cuando la swapchain ya no encaja con
la surface (se redimensionó la ventana). Su hermano `SUBOPTIMAL_KHR` significa "sigue
funcionando, pero ya no es lo ideal". Ambos significan: reconstruye la swapchain (§ 16).

---

## Cómo funciona realmente

### Dónde encaja

```
              § 3                                   § 4
 Entry ─► Instance ─────────────► Surface (a partir de la ventana de winit)
             │   (+ extensiones de surface)  │
             ▼                               │ "¿puede esta queue family presentar aquí?"
      Physical device ◄─────────────────────┘
             │
             ▼
      Logical device (+ VK_KHR_swapchain) ─► Swapchain ─► sus VkImages ─► tus image views
```

Esto cambia el orden de § 3: ahora la instancia necesita las extensiones de surface de la
ventana, y elegir el physical device ahora necesita la surface. Así que **la configuración
de Vulkan tiene que ocurrir después de que exista la ventana** — en `resumed` de `winit`,
justo después de crear la ventana.

### 1. Extensiones de instancia para la ventana

`ash_window::enumerate_required_extensions(display_handle)` devuelve la lista para esta
plataforma (en macOS: `VK_KHR_surface` + `VK_EXT_metal_surface`). Añádelas a la lista de
§ 3.

### 2. Crear la surface

`ash_window::create_surface(...)` recibe el entry, la instancia y los handles de display y
de ventana (la ventana de `winit` implementa los traits de `raw-window-handle`). Para
consultar y destruir surfaces también creas `ash::khr::surface::Instance` — la tabla de
funciones de la extensión de surface.

### 3. Elegir un dispositivo que pueda presentar

Una queue family que puede dibujar no tiene por qué poder presentar en *esta* surface.
Para cada familia candidata, pregunta
`get_physical_device_surface_support(device, family, surface)`. Prefiere una familia que
haga las dos cosas (en GPUs de Apple, normalmente la familia 0 lo hace). Comprueba también
que el dispositivo soporta `VK_KHR_swapchain`.

### 4. Preguntarle a la surface qué hay en el menú

Tres consultas:
- **capabilities** → límites de número de imágenes, extent actual, transformaciones, modos
  de alpha
- **formats** → lista de pares (formato, espacio de color)
- **present modes** → lista de modos

### 5. Elegir

| Ajuste | Elección | Por qué |
|---|---|---|
| Formato | `B8G8R8A8_SRGB` + `SRGB_NONLINEAR` si se ofrece; si no, el primero | Gamma correcta gratis |
| Present mode | `FIFO` (o `MAILBOX` si se ofrece y quieres menos latencia) | FIFO siempre está disponible; vsync va bien para scop |
| Extent | `current_extent` — salvo que sea el valor especial `u32::MAX`, que significa "elige tú": entonces usa el tamaño **físico** de la ventana, limitado a min/max | Debe coincidir con los píxeles reales de la ventana |
| Número de imágenes | `min_image_count + 1`, limitado a `max_image_count` (**0 significa sin máximo**) | Una de repuesto para no esperar al motor de presentación |
| Uso | `COLOR_ATTACHMENT` | Vas a renderizar en ellas |
| Compartición | `EXCLUSIVE` | Las usa una sola queue family |
| Transformación | `current_transform` | Sin rotación |
| Composite alpha | `OPAQUE` si se soporta; si no, el primero soportado | La ventana no es transparente |
| Clipped | `true` | No dibujar píxeles tapados por otras ventanas |

**Composite alpha, descifrado.** El *alpha* es el cuarto valor de un píxel tras rojo, verde
y azul: cuán opaco es (0 = invisible, 1 = sólido). El *compositor* es la parte del sistema
operativo que apila todas las ventanas en la imagen final de pantalla. *Composite alpha* le
dice qué hacer con el alpha de tu imagen cuando pone tu ventana sobre lo que haya detrás:
`OPAQUE` (ignora el alpha — la ventana es sólida), `PRE_MULTIPLIED` / `POST_MULTIPLIED`
(respeta el alpha — ventanas translúcidas; se diferencian en si tus colores ya venían
multiplicados por el alpha), `INHERIT` (que decida el sistema de ventanas). scop quiere una
ventana sólida, así que `OPAQUE`.

Después crea la swapchain a través de `ash::khr::swapchain::Device` (la tabla de funciones
de la extensión de swapchain) y obtén sus imágenes.

### 6. Image views

Para cada imagen de la swapchain: una view 2D, del mismo formato que la swapchain, el
"aspecto" de color, un nivel de mip, una capa. Guárdalas junto a la swapchain.

### 7. Ownership y destrucción

Nuevo orden, espejo de la creación:

**image views → swapchain → device → surface → debug messenger → instance**

Y una regla propia de Rust: **la ventana debe vivir más que la surface.** Si la ventana se
destruye antes, la surface apunta a una ventana muerta. Guarda el contexto de GPU y la
ventana en el mismo struct, con un `Drop` explícito (u orden de campos) que destruya
primero el contexto de GPU.

### Pantallas Retina

La pantalla de tu Mac tiene 2 píxeles físicos por cada punto "lógico". Le pediste a `winit`
una ventana *lógica* de 1280×720; su tamaño físico es 2560×1440. Vulkan trabaja en píxeles
físicos: para el extent usa `window.inner_size()` (físico), nunca el tamaño lógico.

---

## Trampas conocidas

- **Todo se renderiza en el cuarto inferior izquierdo, o borroso** — usaste el tamaño
  lógico en una pantalla Retina. Usa el tamaño físico.
- **Crash o error de validación al minimizar** — el tamaño de la ventana pasa a ser 0×0, y
  una swapchain de 0×0 no es válida. Sáltate el renderizado (y la reconstrucción) mientras
  el tamaño sea cero.
- **`ERROR_OUT_OF_DATE_KHR` tras redimensionar** — esperado; reconstruirás en § 16. Hasta
  entonces, redimensionar puede parar el renderizado. Apúntalo, no te asustes.
- **Error de validación: extensión de swapchain no activada** — olvidaste
  `VK_KHR_swapchain` en las extensiones de dispositivo.
- **"max_image_count es 0, así que pedí 0 imágenes"** — 0 significa *ilimitado*, no "se
  permiten cero". Limita solo cuando no sea cero.
- **Destruir tú las imágenes de la swapchain** — no las posees. Destruye las views y luego
  la swapchain; la swapchain limpia sus imágenes.
- **Ventana destruida antes que la surface** — use-after-free cruzando la frontera
  Rust/C; puede que la validación no lo detecte. El orden de destrucción importa.
- **Los grises se ven lavados o demasiado oscuros** — un desajuste entre un formato
  `_SRGB` y uno `_UNORM` y cómo piensas tú en los valores de color. Ten claro cuál
  elegiste.

---

## Checkpoint

**Cumple:** "dentro de una ventana" a nivel de Vulkan — la ventana ya tiene imágenes en las
que Vulkan puede dibujar. (Método: primero el gate, luego Claude lo escribe y te lo
explica.)

1. `ash-window` (y `raw-window-handle` si hace falta) añadidos al engine, con versión
   fijada.
2. Configuración de Vulkan movida a `resumed`, tras crear la ventana; la instancia recibe
   las extensiones de surface; surface creada a partir de la ventana.
3. La elección de physical device también exige soporte de presentación y
   `VK_KHR_swapchain`.
4. Swapchain creada con las elecciones de la tabla; image views creadas.
5. Imprimir: formato elegido, present mode, extent, número de imágenes.
6. Destrucción limpia en el nuevo orden; la ventana vive más que la surface.

**Verificar:** la ventana se abre; lo impreso muestra un formato razonable, `FIFO` (o
`MAILBOX`), un extent de aproximadamente el doble del tamaño lógico y 2–3 imágenes; al
cerrar no hay mensajes de validación. (La ventana sigue en blanco o con basura — nada se
dibuja hasta § 5.)

---

## Preguntas

Q-4.1 (recuerdo) ¿Para qué sirve la surface, y por qué el núcleo de Vulkan no sabe
simplemente de ventanas?

Q-4.2 (aplicación) La surface reporta `min_image_count = 2` y `max_image_count = 0`.
¿Cuántas imágenes pides, y por qué el 0 no es un problema?

Q-4.3 (predicción) Creas la swapchain con extent 1280×720 porque es lo que le pasaste a
`winit`, en tu MacBook Retina. ¿Qué ves cuando el renderizado funcione, y qué deberías
haber usado?

Q-4.4 (predicción) El usuario minimiza la ventana, y tu código reconstruye la swapchain en
cada evento de redimensionado. ¿Qué sale mal?

Q-4.5 (predicción) En § 3 la configuración de Vulkan se hacía antes de abrir la ventana.
¿Por qué ya no se puede? Nombra los dos pasos que ahora necesitan la ventana.

Respuestas: `ANSWERS.md` § 4
