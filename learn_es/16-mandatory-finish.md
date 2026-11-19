---
section: 16
slug: mandatory-finish
tier: T1
status: done
prereqs: [15]
terms: [swapchain recreation, out of date and suboptimal, size-dependent objects, dependency check, Makefile target, release build, self-evaluation, defence]
---

# Sección 16 — Dejar la parte obligatoria perfecta

**Profundidad: T1** — sin conceptos nuevos: esta es la pasada de acabado. Quedan tres huecos
entre "funciona en mi Mac" y "pasa la evaluación": la ventana no se puede redimensionar
limpiamente, no hay un comando de instalación de una línea, y el README sigue lleno de
huecos. Después revisas todo contra el subject, porque el bonus solo se evalúa si la parte
obligatoria está **perfecta**.
**Serás capaz de:** redimensionar la ventana libremente, darle a cualquiera un clon recién
hecho que compila con `make`, y guiar a un evaluador por cada requisito con pruebas.

---

## La idea

**Redimensionar.** La swapchain (§ 4) es un conjunto de imágenes hechas para *un* tamaño de
ventana. Agranda la ventana y esas imágenes ya no encajan; Vulkan te lo dice la próxima vez
que haces acquire o present ("out of date"), y hasta ahora scop simplemente se saltaba esos
fotogramas. El arreglo es reconstruir la swapchain al nuevo tamaño — y la imagen de
profundidad con ella, que debe coincidir (el explain-back de § 11). Todo lo demás se queda:
los pipelines ponen el viewport y el scissor cada fotograma (§ 6), la cámara lee `aspect`
cada fotograma (§ 10), y a las meshes y texturas la ventana les da igual.

Reconstruir es como cambiarle el lienzo a un pintor: esperar a que el pintor deje el pincel
(la GPU está ociosa), cambiar el lienzo (y su hoja de profundidad a juego), seguir.

**Instalación de una línea.** El subject: *"Proporciona un comando de una línea que compruebe
e instale si es posible todas las dependencias necesarias."* Para scop es `make`: comprobar
que están Rust, el compilador de shaders (`glslc`) y Vulkan, instalar lo que se pueda
instalar, y compilar.

**README.** El subject enumera exactamente lo que debe contener — y un punto importa más de
lo que parece: *"una descripción de cómo se usó la IA — especificando para qué tareas y qué
partes del proyecto."* Mucho del código de scop lo escribió Claude después de tus gates, así
que esto debe ser honesto y concreto. En la defensa, el evaluador comprobará que sabes
explicar esas partes — que es exactamente para lo que ha servido cada explain-back.

**Autoevaluación.** Recorre el subject línea a línea y, para cada requisito, apunta *cómo
demostrar que funciona*. Incluye las cosas que los evaluadores siempre prueban: sus propios
archivos `.obj`, redimensionar, minimizar, cerrar con Esc y con el botón de la ventana.

**Dónde deja de valer la analogía:** un pintor podría seguir con el lienzo viejo; Vulkan no —
a una swapchain out-of-date no se le puede presentar nada.

---

## Los nombres

**swapchain recreation** — destruir la swapchain vieja (imágenes, views, semáforos
`render_done`) y crear una nueva al tamaño actual de la ventana, con el mismo formato y
present mode.

**out of date and suboptimal** — lo que reportan acquire / present tras redimensionar:
`ERROR_OUT_OF_DATE_KHR` = esta swapchain ya no se puede usar; `SUBOPTIMAL_KHR` = sigue
funcionando, pero ya no encaja exactamente con la ventana (p. ej. escalada). scop reconstruye
en los dos casos.

**size-dependent objects** — todo lo que tiene el tamaño de la ventana y se reconstruye con
la swapchain: sus imágenes y views, y la imagen de profundidad. No: los pipelines (viewport
dinámico), la cámara (aspect leído cada fotograma), las meshes, las texturas.

**dependency check** — un script que busca cada herramienta necesaria (`cargo`, `glslc`, el
loader de Vulkan), informa de lo que falta e instala lo que puede instalar con seguridad.

**Makefile target** — una receta con nombre en un `Makefile`: `make` (deps + build),
`make run`, `make clean`. `make` sin nombre ejecuta la primera.

**release build** — `cargo build --release`: optimizado y (en scop) sin la validation layer,
que ralentiza y es una herramienta de desarrollo.

**self-evaluation** — comprobar tú mismo cada requisito del subject, con una forma de
demostrarlo, antes de que lo haga nadie más.

**defence** — la conversación de evaluación: el evaluador ejecuta scop, carga sus propios
archivos y te pide que expliques cualquier parte del código y cualquier decisión.

---

## Cómo funciona realmente

### 1. Redimensionar

```
platform:  WindowEvent::Resized(size)  → apuntar "hay que reconstruir" (salvo 0×0: minimizada)
           draw_frame devuelve OutOfDate o Suboptimal → también "hay que reconstruir"
antes del siguiente fotograma, si "hay que reconstruir":
    device_wait_idle                               // la GPU deja el pincel
    destruir la imagen de profundidad, luego la swapchain (views, semáforos render_done, swapchain)
    crear la swapchain a window.inner_size()       // las mismas funciones choose_* de § 4
    crear la imagen de profundidad al nuevo extent // el mismo DepthImage::new de § 11
```

`device_wait_idle` es la opción sencilla y siempre correcta: redimensionar es raro, así que
pausar la GPU un momento no le cuesta a nadie nada que se note. Los semáforos `image_free` y
las fences pertenecen a los slots de fotograma (§ 5), no a la swapchain, así que sobreviven.

### 2. `make`

```
Makefile
  all:   deps build          ← `make`
  deps:  scripts/deps.sh     ← comprobar + instalar
  build: cargo build --release
  run:   cargo run --release -p scop -- $(OBJ)      (OBJ ?= resources/42.obj)
  clean / fclean / re        ← los nombres habituales de 42

scripts/deps.sh
  falta cargo     → instalar Rust con rustup en tu carpeta personal (sin admin)
  falta glslc     → macOS: brew install shaderc          Linux: ofrecer descargar con curl el SDK de LunarG
                    en tu carpeta personal (sin admin); mientras tanto el build sigue funcionando,
                    desde los .spv de respaldo commiteados (decisión: opción C)
  falta Vulkan    → macOS: brew install molten-vk vulkan-loader   Linux: imprimir el comando
  todo encontrado → "all dependencies OK"
```

En una máquina Linux del campus no puedes instalar paquetes del sistema sin permisos de
administrador, así que el script comprueba y *te lo dice* en vez de fallar en silencio —
"instalar si es posible", como dice el subject.

### 3. README

Exactamente las secciones del subject, más dos que los evaluadores buscarán:

```
*This project has been created as part of the 42 curriculum by luferna3.*
# scop
## Description        — qué es, qué muestra, la separación engine/app
## Instructions       — make, make run OBJ=…, la tabla de controles, cambiar la textura
## Features / technical choices  — breve, apuntando a DECISIONS.md
## Resources          — especificación y tutorial de Vulkan, ash, winit, especificaciones OBJ y TGA, sRGB, …
### AI usage          — plan de aprendizaje y material didáctico; código de checkpoint por sección,
                        escrito por Claude tras cada gate y explicado de vuelta; lo que decidiste,
                        probaste y elegiste tú
```

La sección de IA la firmas tú: Claude la redacta a partir de `PROGRESS.md` y los
checkpoints; tú la editas hasta que cada frase sea verdad con tus propias palabras.

### 4. Lista de autoevaluación

| Requisito del subject | Cómo demostrarlo |
|---|---|
| Parsea el `.obj` y lo renderiza | `make run`, y luego un archivo del evaluador |
| Perspectiva | las partes lejanas más pequeñas; alejar con S |
| Gira sobre su centro, 3 ejes; se traslada | giro + I K / J L / U O; flechas, W S |
| Caras distinguibles, grises sutiles | grises planos al arrancar |
| Una tecla aplica la textura, otra vez la quita, suave | T, y T otra vez a mitad del fundido |
| Textura alegre | tu imagen |
| Sin librerías externas para obj / matemáticas / shaders (/ imágenes) | `Cargo.toml`: solo winit, raw-window-handle, ash, ash-window; `DECISIONS.md` |
| Comando de dependencias de una línea | `make` en un clon recién hecho |
| Formato del README | primera línea, las tres secciones, uso de IA |
| Robusto | otros archivos `.obj` (quads, n-gonos, `vt`/`vn`, `1/2/3`, índices negativos), un archivo roto, redimensionar, minimizar, Esc, botón de cerrar — validación callada en todo momento |

---

## Trampas conocidas

- **Error de validación / crash al redimensionar** — la swapchain destruida mientras la GPU
  aún la usa (sin `device_wait_idle`) (Q-16.2).
- **Imagen estirada tras redimensionar** — swapchain reconstruida pero `aspect` guardado en
  caché en algún sitio, o la imagen de profundidad dejada al tamaño viejo (error de
  validación sobre el render area).
- **Se cuelga tras minimizar** — reconstruir a un tamaño 0×0; espera hasta que la ventana
  vuelva a tener tamaño.
- **`make` solo funciona en tu Mac** — una ruta o herramienta que solo existe ahí (prefijo de
  Homebrew, un `/Users/...` escrito a mano). Prueba en un clon recién hecho; en el campus si
  puedes.
- **`make` no hace nada útil y no avisa** — una herramienta que falta reportada como éxito.
  Cada comprobación debe decir claramente qué falta y cómo conseguirlo.
- **Sección de IA del README demasiado vaga** ("la IA ayudó un poco") — el subject pregunta
  *qué tareas* y *qué partes*. Lo vago parece que esconde algo.
- **El `.obj` de un evaluador revienta scop** — cualquier cosa que haga panic en vez de dar
  un mensaje de error.

---

## Checkpoint

**Cumple (subject):** las restricciones técnicas (comando de una línea), el capítulo del
README, y "parte obligatoria perfecta" para que cuente el bonus. Método: primero el gate,
luego Claude lo escribe y te lo explica.

1. **Engine:** reconstrucción de swapchain + profundidad en `Resized` / out-of-date /
   suboptimal, saltándose el 0×0; validación callada mientras arrastras la ventana.
2. **`Makefile` + `scripts/deps.sh`** como arriba.
3. **README** redactado entero; **tú** editas la sección de uso de IA hasta que sea tuya.
4. **Autoevaluación:** recorre la lista; dos archivos `.obj` de prueba extra en
   `resources/test/` (uno con `vt`/`vn` y tokens `1/2/3`, otro con un n-gono e índices
   negativos) y un archivo roto.

**Verificar:** toda la lista de arriba, en tu Mac — y `make` en un clon recién hecho.

---

## Preguntas

Q-16.1 (recuerdo) ¿Por qué redimensionar la ventana necesita una swapchain nueva? ¿Qué otro
objeto hay que reconstruir con ella, y por qué los pipelines *no* necesitan reconstruirse?

Q-16.2 (predicción) Tras un resultado out-of-date destruyes y recreas la swapchain
enseguida, sin `device_wait_idle`. ¿Qué puede salir mal?

Q-16.3 (aplicación) ¿Qué comprueba `make`, y qué no puede instalar en una máquina Linux del
campus? ¿Por qué eso sigue cumpliendo "comprobar e instalar si es posible"?

Q-16.4 (recuerdo) ¿Qué cuatro cosas debe contener el README, según el subject?

Q-16.5 (aplicación) Un evaluador carga su propio `.obj` con líneas `vt` y `vn`, caras
escritas como `f 1/1/1 2/2/2 3/3/3`, y una cara de 5 esquinas. ¿Qué hace scop con cada una
de esas cosas?

Respuestas: `ANSWERS.md` § 16
