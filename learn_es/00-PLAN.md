# scop — Plan de aprendizaje

**Modo:** 42 (outer core)
**Objetivo:** construir una base en renderizado 3D, más un motor reutilizable
**Inicio:** 2026-11-02
**Stack:** Rust · Vulkan (`ash`, MoltenVK en macOS) · `winit` · GLSL → SPIR-V (`glslc`)
**Idiomas:** learn/ (EN) · learn_es/ (ES)

## Para qué sirve este proyecto
Aprender cómo dibuja realmente una GPU un objeto 3D: desde un fichero de texto con
vértices hasta píxeles sombreados y texturizados en pantalla, usando una API moderna
y explícita. El segundo objetivo es dejar un crate `engine` (inicialización de Vulkan,
matemáticas, carga de mallas e imágenes) lo bastante sólido para reutilizarlo en
ft_newton, ft_vox y ft_minecraft.

## Para qué NO sirve este proyecto
- **Matemáticas en profundidad.** Vectores, matrices y proyección se enseñan para
  "explicar la lógica y usarlas con soltura" (T1). Cuaterniones, estabilidad numérica y
  demostraciones quedan para ft_newton.
- **Iluminación y materiales.** El subject no lo pide. Una iluminación básica podría ser
  un extra del bonus; el renderizado físico (PBR) es para un proyecto posterior.
- **Allocators de memoria de GPU, grabación multihilo, optimización.** scop dibuja un
  objeto; nada aquí mostraría si una optimización importa. ft_vox (miles de chunks) es
  el vehículo adecuado.
- **Arquitectura de motor a gran escala** (ECS, grafos de escena, pipelines de assets).
  El motor se mantiene pequeño; crece cuando un proyecto posterior lo necesite.

## Restricciones (subject v5.0)

**Obligatorio**
- Parsear tú mismo un fichero `.obj` y renderizarlo en una ventana **en perspectiva**.
- Rotar y trasladar el objeto sobre sus tres ejes principales, **con el origen en el
  centro del objeto** (el logo de 42 debe girar sobre su eje central, no sobre una esquina).
- Caras distinguibles por color (por cara del `.obj` o por triángulo); tonos de gris
  para el logo de 42.
- Una tecla activa/desactiva una textura con una **transición suave** (sin corte brusco);
  la textura debe ser algo alegre (ponis, gatitos, unicornios).
- En la defensa se probarán otros ficheros `.obj`.

**Restricciones técnicas**
- Lenguaje: libre (C, C++ o Rust preferidos). API: OpenGL, Vulkan o Metal.
- Un comando de una línea que compruebe e instale todas las dependencias (p. ej. un Makefile).
- Librerías externas **solo** para gestión de ventanas y eventos.
- **Sin librerías externas** para cargar `.obj`, matrices o cargar shaders.
  (La carga de imágenes no aparece explícitamente, pero escribir tú el cargador de
  texturas es la interpretación esperada — trátalo como prohibido.)

**README** — primera línea en cursiva: *This project has been created as part of the 42
curriculum by luferna3.* Más Description, Instructions, Resources (con uso de IA).

**Bonus** (solo se evalúa si la parte obligatoria es perfecta)
- [10pt] `.obj` complejos: caras cóncavas y no coplanares; renderizar la tetera original
  sin artefactos en los bordes.
- [5pt] Textura sin estiramiento visible en algunas caras.
- [10pt] Tus propios extras — **se deciden más adelante** (§ 19).

## Reparto de profundidad

Puntuación 1–3 en centralidad, transferibilidad y coste de aprenderlo más tarde. El
objetivo es "construir una base", así que los conceptos con transferibilidad 3 reciben +1.
Ajustes de Lucas aplicados: matemáticas limitadas a T1; punteros y ventanas ya conocidos
por C/C++, raylib y mlx.

| Concepto | Cent. | Transf. | Coste | Total | Nivel |
|---|---|---|---|---|---|
| Ownership, borrowing, `Result`, `Drop` en Rust | 2 | 3 | 3 | 9 | T2 |
| `unsafe`, punteros crudos, `ash` | 3 | 2 | 2 | 7 | T1 (punteros conocidos) |
| Cómo dibuja una GPU (pipeline de renderizado) | 3 | 3 | 3 | 10 | T2 |
| Instancia, dispositivo y colas de Vulkan | 3 | 3 | 2 | 9 | T2 |
| Surface + swapchain | 3 | 3 | 2 | 9 | T2 |
| Command buffers + sincronización | 3 | 3 | 3 | 10 | T2 |
| Shaders, GLSL → SPIR-V, pipeline gráfico | 3 | 3 | 3 | 10 | T2 |
| Buffers + memoria de GPU | 3 | 3 | 3 | 10 | T2 |
| Arquitectura motor/aplicación (crate reutilizable) | 2 | 3 | 3 | 9 | T2 (objetivo declarado) |
| Vectores, matrices, perspectiva (librería propia) | 3 | 3 | 2 | 9 | **T1** (ajuste) |
| Uniforms, push constants, descriptors | 3 | 3 | 2 | 9 | T2 |
| Depth buffer | 3 | 3 | 2 | 9 | T2 |
| Parser de `.obj` | 3 | 2 | 1 | 6 | T1 |
| Ventana + input (`winit`) | 3 | 2 | 1 | 6 | T1 (conocido por mlx/raylib) |
| Colores por cara, controles, centrado | 3 | 1 | 1 | 5 | T1 |
| Cargador de imágenes propio (TGA) | 2 | 2 | 1 | 5 | T1 |
| Texturas, samplers, UVs | 3 | 3 | 2 | 9 | T2 |
| Transición suave color↔textura | 3 | 1 | 1 | 5 | T1 |
| Bonus: caras cóncavas/no coplanares | 2 | 2 | 2 | 6 | T1 |
| Bonus: mapeado de textura sin estiramiento | 2 | 2 | 2 | 6 | T1 |

**Nota sobre el vehículo:** scop es un vehículo excelente para **el modelo de objetos de
Vulkan y el pipeline de renderizado** — un solo objeto, sin distracciones. Es un mal
vehículo para el **rendimiento** (una malla), la **profundidad matemática** (una cámara y
una matriz de modelo) y la **iluminación**. Eso queda a propósito para ft_newton y ft_vox.

## Secciones

| # | Fichero | Título | Nivel | Estado | Checkpoint |
|---|---|---|---|---|---|
| 1 | `01-rust-for-c-programmers.md` | Rust para quien viene de C | T2 | sin empezar | Workspace (`engine` + `scop`), lectura de fichero con `Result` |
| 2 | `02-how-a-gpu-draws.md` | Cómo dibuja una GPU un triángulo | T2 | sin empezar | Ventana `winit`, event loop, Esc para salir |
| 3 | `03-vulkan-instance-and-device.md` | Hablar con la GPU | T2 | sin empezar | Instancia + validation layers + dispositivo lógico |
| 4 | `04-surface-and-swapchain.md` | Llevar imágenes a la pantalla | T2 | sin empezar | Swapchain creada, imágenes listadas |
| 5 | `05-commands-and-sync.md` | Dar trabajo a la GPU, y esperar | T2 | sin empezar | Ventana limpiada a un color cada fotograma |
| 6 | `06-shaders-and-pipeline.md` | Programas que se ejecutan en la GPU | T2 | sin empezar | Triángulo fijo en pantalla |
| 7 | `07-buffers-and-gpu-memory.md` | Enviar datos a la GPU | T2 | sin empezar | Triángulo/quad desde vertex + index buffers |
| 8 | `08-engine-architecture.md` | Separar motor y aplicación | T2 | sin empezar | Refactor: scop solo usa la API pública del motor |
| 9 | `09-maths-for-3d.md` | Las matemáticas 3D justas | T1 | sin empezar | `Vec3`/`Mat4` propios + tests |
| 10 | `10-uniforms-and-descriptors.md` | Pasar datos por fotograma a los shaders | T2 | sin empezar | Cubo girando en perspectiva |
| 11 | `11-depth-buffer.md` | Qué está delante de qué | T2 | sin empezar | Cubo con oclusión correcta |
| 12 | `12-obj-parser.md` | Leer ficheros `.obj` | T1 | sin empezar | El logo de 42 y la tetera cargan y se renderizan |
| 13 | `13-faces-and-controls.md` | Caras grises y mover el modelo | T1 | sin empezar | Grises por cara, rotar/trasladar sobre el centro |
| 14 | `14-textures.md` | Imágenes sobre superficies | T2 | sin empezar | Cargador TGA propio, modelo texturizado |
| 15 | `15-smooth-transition.md` | Fundir color y textura | T1 | sin empezar | Tecla con transición suave |
| 16 | `16-mandatory-finish.md` | Dejar la parte obligatoria perfecta | T1 | sin empezar | Redimensionado, `make` de una línea, README, autoevaluación |
| 17 | `17-bonus-complex-obj.md` | Caras cóncavas y no planas | T1 | sin empezar | La tetera original se ve limpia |
| 18 | `18-bonus-texture-mapping.md` | Texturas sin estiramiento | T1 | sin empezar | Ninguna cara estirada |
| 19 | `19-bonus-extras.md` | Tus propios extras | por decidir | sin empezar | Se decide más adelante |

Las secciones 3–19 se escriben cuando llegues a ellas, para que el plan se adapte a lo
que resulte difícil.

## Preparación de la defensa
<!-- Se rellena según se completan secciones. Fuente: DECISIONS.md + trampas de cada sección. -->

## Aplazado / v2
- Iluminación (candidata a extra del bonus, se decide en § 19).
- Ajuste del present mode de la swapchain, más de 2 fotogramas en vuelo.
