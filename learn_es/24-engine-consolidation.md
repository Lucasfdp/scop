---
section: 24
slug: engine-consolidation
tier: T1
status: done
prereqs: [8, 18, 19, 20, 21, 22, 23]
terms: [consolidation, reuse test, default shaders, shader override, hard-coded vs setting, mesh builder, colour callback, tween]
---

# Sección 24 — Llevar las funciones del bonus al engine

**Profundidad: T1, sin matemáticas.** Casi nada nuevo que aprender: esta sección *mueve*
código. La habilidad está en decidir **dónde va cada pieza**, y en hacer la versión del engine
lo bastante general para que el siguiente proyecto pueda usarla sin cambios. Petición del
propietario, 2026-11-21: "todo lo reutilizable del bonus va al engine", como hizo § 8 con la
parte obligatoria.
**Serás capaz de:** clasificar cualquier funcionalidad en engine o app con dos preguntas,
detectar las suposiciones que impedirían reutilizarla, y dejar scop como "solo las decisiones
del subject".

---

## La idea

**Dos preguntas, las mismas que en § 8.** Para cada pieza de scop:

1. **¿La querrían ft_newton, ft_vox o el engine en Rust, sin cambios?** → engine.
2. **¿Es una decisión que tomó el subject (o scop)?** → scop.

Una cámara orbital pasa la prueba 1: todo proyecto 3D quiere una. Las caras grises no: existen
porque el subject pide grises distinguibles — esa es una decisión de scop (Q-24.1).

**Lo difícil es "sin cambios".** El código escrito para scop da por hecho, sin decirlo, el mundo
de scop. La cámara orbital da por hecho que "arriba" es +y; en el mundo de ft_newton arriba
podría ser +z. Llevarla al engine significa convertir cada suposición en un **ajuste** con el
valor de scop por defecto, no copiarla tal cual (Q-24.5).

**Shaders por defecto.** La iluminación, el cromado, los colores de depuración y el cielo viven
hoy en los archivos de shader de scop. Decisión del propietario: el engine los incluye como
**shaders por defecto**. Una app los obtiene pidiendo un pipeline por defecto; una app que
quiera otra cosa sigue escribiendo sus propios shaders (un **override**) y dibujando con
`draw_with`, exactamente como hoy. Los push params de los shaders por defecto pasan a tener
significados *con nombre* (blend, modo de color, rango de profundidad) en un pequeño tipo del
engine, para que las apps dejen de empaquetar cuatro números sueltos a mano (Q-24.3).

**Partir una funcionalidad por la mitad.** Algunas funcionalidades son mitad generales, mitad de
scop. El paso `.obj` → mesh es general (separar objetos, normales planas, UVs con box mapping);
el gris por cara es de scop. Así que el **constructor de meshes** del engine recibe el color
como **callback**: "para la cara *n*, ¿qué color?" — scop pasa sus grises, ft_newton pasa los
suyos. La recarga en caliente se parte igual: el engine nota los guardados y compila; la app
decide cuáles de *sus* pipelines reciben los shaders nuevos (Q-24.4).

**Lo que se queda scop:** las asignaciones de teclas y el texto de controles, la regla de los
grises por cara, qué imagen recibe cada objeto (N), la disposición 8 × 8 × 8 de la multitud, la
redacción del título de la ventana, `main()`.

**Dónde deja de valer la analogía:** "llevarlo al engine" suena a cortar y pegar. La mayor parte
del trabajo es lo contrario — decidir qué *no* mover, y qué valores se convierten en ajustes.

---

## Los nombres

**consolidation** — reunir código reutilizable y disperso en el único sitio que usa todo
proyecto.

**reuse test** — las dos preguntas de arriba: ¿se quiere sin cambios en otro sitio? ¿es una
decisión que tomó esta app?

**default shaders** — shaders que el engine incluye y compila él mismo (iluminación, cromado,
colores de depuración, cielo), usados mediante `Gfx::create_default_pipeline` y similares.

**shader override** — los shaders propios de una app usados en vez de los de por defecto; al
engine le da igual cuáles.

**hard-coded vs setting** — un valor escrito en el código (el +y arriba de scop) frente a uno
que elige quien llama, con un valor por defecto razonable.

**mesh builder** — el paso `.obj` → meshes del engine: una mesh por objeto, esquinas propias por
triángulo, normales planas, UVs con box mapping opcionales, color desde un callback.

**colour callback** — una función que la app le pasa al engine ("cara n → color"), para que el
engine siga siendo general y la app conserve su regla. En Rust: un closure, `|face| …`.

**tween** — un valor que se suaviza hacia un objetivo con el tiempo (el fundido de § 15,
generalizado).

---

## Cómo funciona realmente

### Qué se mueve, y en qué se convierte

| Hoy (scop) | Engine, después | Ajuste en vez de valor fijo |
|---|---|---|
| `Orbit`, zoom, distancia de encuadre, near/far, `depth_range` | `camera.rs`: `OrbitCamera` | eje de arriba, límite de pitch, límites de zoom, sensibilidad |
| `Placement` (+ su matriz alrededor de un centro) | `transform.rs`: `Transform` | — |
| `Fade` + `smoothstep` | `tween.rs`: `Tween` | duración, suavizado |
| `object_meshes`, `face_normals`, `box_uv`, `main_axis` | `mesh_builder.rs` | mapeo de UV (ninguno / box), callback de color |
| `hot_reload.rs` | `shader_watch.rs` | la lista de archivos |
| `sky.rs`, `sky_box`, `load_sky` | `sky.rs` | dirección del sol, colores, tamaño de cara |
| `mesh.vert/frag`, `sky.vert/frag` | `engine/shaders/` (compilados por el propio `build.rs` del engine) | push params con nombre en `DefaultParams` |
| `load_model` (leer + parsear + construir) | `mesh_builder::load_obj` | callback de color, mapeo de UV |

### Lo que necesitan los shaders por defecto

```
engine/build.rs        compila engine/shaders/*.vert|frag (y el respaldo .spv commiteado, § 16)
Gfx::create_default_pipeline(style)       // relleno / wireframe / puntos, shaders por defecto
Gfx::create_default_sky()                 // pipeline de fondo + caja + cubo generado
DefaultParams { blend, colour_mode, depth_range } → los cuatro números de push
ColourMode { Normal, Facing, Depth, Chrome }
recarga en caliente de los shaders por defecto: el engine vigila engine/shaders/
```

### Decisiones

| Elección | Frente a | Por qué |
|---|---|---|
| **Ajustes con los valores de scop por defecto** | Copiar las suposiciones de scop | Reutilizar significa *usar* sin cambios, no *valores* sin cambios |
| **Color como callback** | Grises en el engine / sin colores | La regla sigue siendo de scop; el mecanismo es general |
| **Shaders por defecto + overrides** | Que cada app escriba sus shaders / que el engine imponga los suyos | Los proyectos funcionan desde el principio; cualquiera puede sustituirlos |
| **`DefaultParams` da nombre a los números de push** | `[f32; 4]` sueltos | Solo para los shaders por defecto; los shaders propios siguen con `draw_with` en crudo |
| **El engine nota y compila, la app elige pipelines** | Que el engine reconstruya los pipelines por su cuenta | Solo la app sabe cuáles de sus pipelines usan qué shaders |
| **La disposición de la multitud se queda en scop** | Un helper de cuadrícula en el engine | Cada proyecto coloca sus copias a su manera (propietario, 2026-11-21) |
| **Los tests se mueven con su código** | Reescribir los tests | Demuestran que nada cambió en el traslado |

---

## Trampas conocidas

- **El comportamiento cambia en el traslado** — un valor por defecto que no es el antiguo de scop
  (límite de pitch, pasos de zoom). Los tests que se mueven con el código pillan la mayoría;
  luego compruébalo a ojo.
- **El engine empieza a saber de scop** — una constante como `LIGHT_DIRECTION` o la regla de los
  grises acabando en el engine. Pasa la prueba de reutilización a cada constante.
- **Dos copias de un shader** — la de scop y la del engine se van separando. Tras el traslado,
  scop no tiene archivos de shader propios (usa los de por defecto).
- **La recarga en caliente vigila la carpeta equivocada** — ahora debe vigilar
  `engine/shaders/`.
- **Push params con dos significados** — los shaders por defecto son dueños de su significado;
  los shaders propios usan números en crudo. Nunca mezcles los dos en un pipeline.

---

## Checkpoint

**Cumple (subject):** nada nuevo — el mismo scop, más limpio por dentro. Método: primero el gate,
luego Claude lo escribe y te lo explica.

1. **Engine:** `camera.rs`, `transform.rs`, `tween.rs`, `mesh_builder.rs`, `shader_watch.rs`,
   `sky.rs`; `engine/shaders/` + `engine/build.rs`; `create_default_pipeline`,
   `create_default_sky`, `DefaultParams`, `ColourMode`. Cada elemento público nuevo documentado,
   nada con forma de scop dentro.
2. **scop:** usa solo la API pública del engine para todo ello; se queda teclas, grises, rotación
   de imágenes, disposición de la multitud, título, `main()`. Sin archivos de shader propios.
3. **Tests:** cada test movido pasa en su nuevo sitio; un test que usa un ajuste no por defecto
   (p. ej. +z arriba) para la cámara.

**Verificar:** scop se comporta exactamente como al final de § 23 — cada tecla, multitud,
cromado, cielo, recarga en caliente (ahora sobre `engine/shaders/`); validación callada;
`main.rs` mucho más corto.

---

## Preguntas

Q-24.1 (aplicación) Pasa las dos preguntas de reutilización a `face_grey`. ¿Engine o scop — y qué
parte de construir meshes con color *sí* va al engine?

Q-24.2 (aplicación) ¿Por qué el `OrbitCamera` del engine debería ser dueño del límite de pitch, en
vez de dejar que cada app se acuerde de ponerlo?

Q-24.3 (aplicación) Los shaders por defecto leen los push params como blend, modo de color y rango
de profundidad. Tu shader de ft_newton quiere esos cuatro números para otra cosa. ¿Qué haces, y por
qué no reutilizar sin más el pipeline por defecto?

Q-24.4 (aplicación) La recarga en caliente está partida: el engine nota los guardados y compila, la
app decide qué pipelines reconstruir. ¿Por qué el engine no puede hacer la segunda parte por su
cuenta?

Q-24.5 (predicción) En el mundo de ft_newton +z es arriba. Llevas el `Orbit` de scop al engine
exactamente como está. ¿Qué sale mal cuando ft_newton lo usa, y qué debería haber sido un ajuste?

Respuestas: `ANSWERS.md` § 24
