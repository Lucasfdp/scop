---
section: 2
slug: how-a-gpu-draws
tier: T2
status: not-started
prereqs: [1]
terms: [CPU, GPU, API, Vulkan, MoltenVK, rendering pipeline, vertex, primitive, vertex shader, rasterisation, fragment, fragment shader, framebuffer, swapchain, NDC, GLSL, SPIR-V, command buffer, queue, winit, event loop]
---

# Sección 2 — Cómo dibuja una GPU un triángulo

**Profundidad: T2** — cada objeto de Vulkan de las secciones 3–11 existe para configurar
una etapa del pipeline descrito aquí. Si puedes dibujar este pipeline de memoria, Vulkan
deja de ser una lista de 40 objetos misteriosos y pasa a ser "los mandos de la etapa X".
**Serás capaz de:** explicar, sin apuntes, qué pasa entre "aquí tienes tres puntos" y "hay
un triángulo en pantalla", y por qué Vulkan te obliga a configurar tanto. Además tendrás
una ventana abierta — donde Vulkan va a dibujar.

---

## La idea

Con mlx dibujabas escribiendo tú los píxeles en una imagen, en el procesador: un bucle
sobre cada píxel decidiendo su color. Funciona, pero el procesador hace cada píxel uno tras
otro. Una ventana de 1920×1080 son dos millones de píxeles, sesenta veces por segundo.

Una tarjeta gráfica funciona distinto. Es como una fábrica con miles de operarios,
individualmente lentos y no muy listos, pero que hacen todos *el mismo trabajo pequeño a la
vez*. No le dices "colorea el píxel 1, luego el 2". Le dices: "esta es la *receta* para
colorear un píxel; estos son los datos; adelante". Cada operario ejecuta la receta sobre su
propio píxel, en paralelo.

Dibujar un objeto 3D es una cadena de montaje con puestos fijos:

1. **Entran puntos.** Das una lista de esquinas — para un cubo, sus ocho esquinas, cada una
   como `(x, y, z)` más extras como un color.
2. **Colocar cada punto.** Un pequeño programa que escribes tú se ejecuta *una vez por
   punto* y responde a una pregunta: "¿dónde cae este punto en la pantalla?". Aquí ocurren
   la rotación y la perspectiva.
3. **Unir los puntos.** Los puntos se agrupan de tres en tres en triángulos. En una GPU
   todo está hecho de triángulos, porque tres puntos siempre están en una superficie plana.
4. **Encontrar los píxeles cubiertos.** Hardware fijo calcula qué píxeles cubre cada
   triángulo. Esta parte no la escribes tú.
5. **Colorear cada píxel.** Un segundo programa que escribes tú se ejecuta *una vez por
   píxel cubierto* y responde: "¿de qué color es este píxel?". Gris plano, o una consulta a
   una foto de un gatito.
6. **Quedarse con el más cercano.** Si dos triángulos cubren el mismo píxel, se queda el más
   cercano a la cámara (§ 11).
7. **Mostrarlo.** La imagen terminada se entrega a la pantalla, mientras la siguiente ya se
   está dibujando en una segunda imagen.

Librerías como raylib esconden esta cadena tras `DrawModel()`. Vulkan te da el plano
entero de la fábrica: configuras tú cada puesto, por adelantado.

**Dónde deja de valer la analogía de la fábrica:** en una fábrica los puestos trabajan uno
tras otro sobre una pieza. En una GPU todos los puestos trabajan a la vez sobre triángulos y
fotogramas *distintos*, y el procesador que dio las órdenes ya ha pasado a otra cosa. Ese
segundo hecho es la raíz de § 5.

---

## Los nombres

**CPU** — **C**entral **P**rocessing **U**nit (unidad central de procesamiento). El
procesador principal: pocos núcleos, muy rápidos y muy listos. Ejecuta tu programa de Rust.

**GPU** — **G**raphics **P**rocessing **U**nit (unidad de procesamiento gráfico). Miles de
núcleos simples ejecutando el mismo programa sobre datos distintos. En tu Mac, la GPU está
en el mismo chip que la CPU (Apple Silicon), pero sigue siendo un procesador aparte al que
le mandas trabajo.

**API** — **A**pplication **P**rogramming **I**nterface (interfaz de programación de
aplicaciones). El conjunto de funciones que te ofrece una librería o sistema. Vulkan es una
API: una lista de funciones de C que implementa el driver de la GPU.

**Vulkan** — la API gráfica explícita y de bajo nivel del Khronos Group (que también
mantiene OpenGL), publicada en 2016. Nombrada por Vulcano, dios romano del fuego y la forja;
nació como la API "Mantle" de AMD. "Explícita" significa que el driver no adivina nada: tú
indicas cada ajuste.

**MoltenVK** — una capa de traducción que implementa Vulkan encima de Metal, la API de
Apple, porque macOS no tiene driver nativo de Vulkan. "Molten" (fundido) + "VK" (la
abreviatura habitual de Vulkan): Vulkan fundido en Metal. Por eso funcionó `vkcube`.

**rendering pipeline** (pipeline de renderizado) — la cadena de montaje de arriba: el orden
fijo de etapas que convierte vértices en píxeles. "Pipeline" (tubería) porque cada etapa
alimenta a la siguiente y todas trabajan a la vez sobre piezas distintas.

**vertex** (vértice; plural *vertices*) — una esquina más los datos que le asocies (color,
coordenadas de textura). Del latín, "punto de giro" o "cima".

**primitive** (primitiva) — la forma básica en la que se ensamblan los vértices. Para ti,
siempre un triángulo. Se llama así porque es la unidad dibujable más pequeña.

**vertex shader** — el pequeño programa que se ejecuta una vez por vértice (etapa 2). Su
salida principal es la posición del vértice en pantalla.

**shader** en general — un programa que se ejecuta en la GPU. El nombre es histórico: los
primeros solo calculaban el *sombreado* (shading: luces y sombras). Hoy calculan cualquier cosa.

**rasterisation** (rasterización) — la etapa 4: convertir un triángulo (una forma
matemática) en la cuadrícula de píxeles que cubre. De *raster*, la cuadrícula de líneas que
dibujaba una pantalla CRT antigua; a su vez del latín *rastrum*, "rastrillo".

**fragment** (fragmento) — un píxel candidato producido por la rasterización: "este triángulo
cubre este píxel, aquí están los datos interpolados". Solo se convierte en píxel real si
supera el test de profundidad. Por eso se llama *fragmento* y no *píxel*.

**fragment shader** — el programa que se ejecuta una vez por fragmento (etapa 5) y devuelve
su color. (OpenGL y Vulkan dicen "fragment shader"; DirectX lo llama "pixel shader".)

**framebuffer** — la(s) imagen(es) en la(s) que se dibuja: una imagen de color y, más
adelante, una de profundidad. Un buffer de memoria que contiene un fotograma (frame).

**swapchain** — un pequeño conjunto (normalmente 2–3) de imágenes que se turnan: la
pantalla muestra una mientras la GPU dibuja en otra, y luego se intercambian. Una *cadena*
(chain) de imágenes que se *intercambian* (swap).

**NDC** — **N**ormalised **D**evice **C**oordinates (coordenadas normalizadas de
dispositivo). El sistema de coordenadas en el que acaba la salida del vertex shader: una caja
donde x e y van de −1 a +1 a lo ancho de la ventana, sea cual sea su tamaño en píxeles.
"Normalizadas" porque el rango es fijo; "de dispositivo" porque no dependen de la ventana.
**En Vulkan, +y apunta hacia abajo** (en OpenGL apunta hacia arriba), y la profundidad va de 0 a 1.

**GLSL** — **O**pen**GL** **S**hading **L**anguage (lenguaje de sombreado de OpenGL), el
lenguaje parecido a C en el que escribirás los shaders. Pese al nombre, sirve también para Vulkan.

**SPIR-V** — **S**tandard **P**ortable **I**ntermediate **R**epresentation, versión 5 ("V")
(representación intermedia portable estándar). Un formato binario compilado para shaders.
Vulkan solo acepta SPIR-V, nunca texto GLSL, así que compilas GLSL → SPIR-V con `glslc`
antes de ejecutar — como compilar C a un fichero objeto. Cargar esos bytes en Vulkan es la
parte que debes escribir tú.

**command buffer** — una lista grabada de instrucciones para la GPU ("usa este pipeline,
usa estos vértices, dibuja 36 vértices"). La grabas en la CPU y luego la envías. Como
escribir la lista de la compra y entregarla, en lugar de llamar por teléfono por cada producto.

**queue** (cola) — donde envías los command buffers. La GPU trabaja su cola a su ritmo, de
forma **asíncrona**: la llamada de envío vuelve al instante, mucho antes de que la GPU termine.

**winit** — **win**dow **init**ialisation (inicialización de ventanas). El crate de Rust que
crea una ventana y entrega eventos de teclado, ratón y redimensionado. Tu equivalente a mlx.

**event loop** (bucle de eventos) — el bucle que espera a que pasen cosas (tecla pulsada,
ventana redimensionada, clic en cerrar) y llama a tu código para cada una. Le cedes el
control; te llama de vuelta, exactamente como `mlx_loop` con hooks.

---

## Cómo funciona realmente

### El pipeline, con los nombres reales

```
 Lado CPU (tu Rust)                         Lado GPU
 ───────────────────                        ────────────────────────────────────────
 datos de vértices ──► vertex buffer ────►  [Vertex shader]  una vez por vértice
 (del .obj)            (memoria GPU, § 7)        │  devuelve la posición en clip space
                                                 ▼
 datos por fotograma (rotación, tiempo) ──► [Primitive assembly]  vértices → triángulos
 (uniforms, § 10)                                │
                                                 ▼
                                            [Rasterisation]  triángulo → fragmentos   (fijo)
                                                 │  interpola las salidas de los vértices
                                                 ▼
 imagen de textura (§ 14) ────────────────► [Fragment shader]  una vez por fragmento
                                                 │  devuelve un color
                                                 ▼
                                            [Depth test]  quedarse el más cercano (§ 11) (fijo)
                                                 │
                                                 ▼
                                            imagen de la swapchain ──► pantalla (§ 4)
```

Las dos cajas que **programas** son el vertex shader y el fragment shader (§ 6). Las demás
son de **función fija** (fixed function): las configuras con ajustes (¿descartar caras
traseras? ¿test de profundidad?), pero no escribes su código.

### Interpolación — la magia silenciosa de la rasterización

Da a las tres esquinas de un triángulo los colores rojo, verde y azul. Al fragment shader no
le llega "rojo" o "verde" — le llega una *mezcla*, ponderada según lo cerca que esté el
píxel de cada esquina. Cualquier valor que devuelva el vertex shader se mezcla así a lo largo
del triángulo. Así llegan las coordenadas de textura a cada píxel (§ 14), y así conseguirás
la transición suave de § 15. Para colores planos por cara (§ 13) tendrás que *desactivar*
esto — tenlo en mente.

### Espacios de coordenadas, una línea cada uno (las matemáticas son § 9)

Un vértice pasa por varios sistemas de coordenadas antes de llegar a la pantalla:

1. **Model space** — tal como está en el `.obj`, relativo al origen del propio modelo.
2. **World space** — tras colocar/rotar el modelo en la escena.
3. **View space** — relativo a la cámara.
4. **Clip space** — tras la proyección en perspectiva; es lo que devuelve el vertex shader,
   como cuatro números `(x, y, z, w)`.
5. **NDC** — clip space dividido por `w`. Esta división es lo que hace pequeño lo lejano.
6. **Pantalla** — NDC estirado al tamaño en píxeles de la ventana. Lo hace hardware fijo.

### Por qué Vulkan necesita tantos objetos

OpenGL es una gran máquina de estados global: pones "color actual = rojo" y el driver se
encarga del resto, a menudo parándose a recompilar cosas a tus espaldas. Vulkan te pide
declararlo todo por adelantado para que el driver nunca tenga que adivinar. Este es el mapa
completo de objetos que conocerás — **no los aprendas ahora**, solo fíjate en que cada uno
pertenece a un puesto:

| Objeto | Para qué sirve | Sección |
|---|---|---|
| Instance | Tu conexión con la librería de Vulkan | 3 |
| Physical device | Una GPU real de la máquina | 3 |
| Logical device + queue | Tu sesión con esa GPU, y su bandeja de entrada | 3 |
| Surface + swapchain | Las imágenes de la ventana donde dibujar | 4 |
| Command pool + buffer | Las listas de la compra de trabajo para la GPU | 5 |
| Fence + semaphore | Señales de "espera a que la GPU termine" | 5 |
| Shader module + pipeline | Los dos programas + todos los ajustes fijos, horneados juntos | 6 |
| Buffer + device memory | Datos de vértices/índices en la GPU | 7 |
| Descriptor set | Cómo encuentran los shaders los uniforms y las texturas | 10 |
| Image + image view + sampler | Depth buffer y texturas | 11, 14 |

### La ventana: `winit`

Vulkan no crea ventanas. Las crea `winit`, que más tarde entrega a Vulkan un **window
handle** (con el crate `raw-window-handle`) para que Vulkan pueda dibujar en ella.

Desde `winit` 0.30, implementas un trait (`ApplicationHandler`) sobre tu propio struct, y el
event loop llama a sus métodos: uno cuando la app está lista para crear su ventana
(`resumed`) y otro por cada evento de ventana (`window_event` — teclas, redimensionado,
petición de cierre). Lee la documentación de la **versión exacta que fijes** — la API ha
cambiado entre versiones y la mayoría de ejemplos en internet son de versiones antiguas.

---

## Trampas conocidas

- **Pensar que la GPU ejecuta tu comando cuando lo llamas.** El envío vuelve al instante; la
  GPU puede seguir con el fotograma anterior. Si cambias datos que la GPU aún está leyendo,
  verás parpadeos o basura. Se arregla en § 5.
- **Esperar que Vulkan acepte GLSL.** `vkCreateShaderModule` quiere bytes de SPIR-V. Pasarle
  texto GLSL da un error de validación o un crash.
- **Modelos boca abajo.** En NDC de Vulkan +y apunta hacia abajo; la mayoría de tutoriales de
  OpenGL suponen +y hacia arriba. Si tu modelo sale invertido, es por esto (se arregla en la
  matriz de proyección, § 10).
- **Ejemplos antiguos de `winit`.** El código con closures `EventLoop::run(move |event, ...|`
  es la API anterior a 0.30. Si no compila, mira la versión.
- **Ventana creada fuera de `resumed`.** En `winit` reciente, crear la ventana antes de que el
  event loop esté en marcha está obsoleto (y falla en algunas plataformas).

---

## Checkpoint

**Cumple:** "dentro de una ventana", y la mitad de input de "rotar y trasladar con cualquier
método de entrada".

1. Añade `winit` como dependencia del crate **engine**. Fija una versión
   (p. ej. `winit = "=0.30.x"` con el parche actual).
2. En `engine`, crea un módulo `platform` (o `window`) dueño del event loop y de la ventana.
   El binario scop no debe importar `winit` directamente — llama al motor. (Es la primera
   decisión de frontera motor/aplicación; regístrala en `DECISIONS.md` si eliges otra cosa.)
3. Abre una ventana titulada `scop`, de 1280×720.
4. Gestiona: botón de cerrar → salir limpiamente; `Esc` → salir limpiamente; ventana
   redimensionada → imprimir el nuevo tamaño; cualquier otra tecla → imprimir cuál.
5. Mantén el comportamiento de § 1: la ruta del `.obj` se sigue leyendo primero de los
   argumentos, y una ruta mala termina *antes* de abrir la ventana.

**Verifica:** la ventana se abre; al redimensionar se imprimen tamaños; `Esc` y el botón de
cerrar salen con código 0; `cargo clippy` sin warnings.

---

## Preguntas

Q-2.1 (recall) ¿Para qué dos etapas del pipeline escribes programas, y con qué frecuencia se
ejecuta cada una?

Q-2.2 (application) Un cubo tiene 8 esquinas, pero enviarás 36 vértices para dibujarlo. ¿De
dónde sale el 36?

Q-2.3 (prediction) Tu vertex shader devuelve las tres esquinas de un triángulo con colores
rojo, verde y azul, y tu fragment shader solo devuelve el color que recibe. ¿Cómo se ve el
triángulo en pantalla y qué etapa es responsable?

Q-2.4 (prediction) Grabas un command buffer, lo envías y en la siguiente línea de Rust
sobrescribes los datos de vértices que usa. ¿Qué podrías ver, y por qué?

Q-2.5 (prediction) Coges un vertex shader de un tutorial de OpenGL, sin cambios, y
renderizas el modelo de una persona de pie. ¿Qué esperas ver en Vulkan?

Respuestas: `ANSWERS.md` § 2
