---
section: 8
slug: engine-architecture
tier: T2
status: done
prereqs: [7]
terms: [public API, crate boundary, coupling, inversion of control, App trait, default method, generic function, newtype, typed handle, draw list, input abstraction, delta time, re-export]
---

# Sección 8 — Separar el engine de la app

**Profundidad: T2** — este es tu segundo objetivo declarado: un crate `engine` que puedas
llevarte a ft_newton, ft_vox y ft_minecraft. Una frontera desordenada ahora significa
reescribir el engine para cada proyecto; una limpia significa que cada proyecto solo
escribe lo nuevo. También es lo primero que ve un evaluador al abrir el repo.
**Serás capaz de:** decir qué va en el engine y qué va en la app, diseñar una pequeña API
pública en torno a eso y refactorizar scop para que no use *nada más* que esa API.

---

## La idea

Mira cómo se hablan hoy scop y el engine:

```rust
platform::run(config, MESH_SHADERS, quad(), print_gpu_info, pulsing_blue)
```

Cinco argumentos, cada uno añadido por una sección. Y por dentro, el engine sabe cosas que
no debería: construye exactamente **un** pipeline y sube exactamente **una** mesh al
arrancar, y `draw_frame` dibuja esa única mesh. ft_newton necesita docenas de objetos
moviéndose cada fotograma; esta API no puede hacerlo sin reescribirse.

Piensa en el engine como una **compañía de teatro** que contratas, y en cada proyecto como
una **obra**.

- La compañía pone **el edificio y el equipo técnico**: el escenario (ventana), la
  iluminación (configuración de la GPU), los cambios de escena (swapchain, fotogramas,
  sincronización). Toda obra los necesita; ninguna obra debería reconstruirlos.
- La obra pone **el guion**: qué hay en escena, qué pasa cuando alguien pulsa una tecla, de
  qué color es el telón de fondo.
- La compañía lleva la función. No espera a que el guion diga "ahora refresca las luces";
  **llama a los actores cuando les toca**: "prepara tu atrezo", "el público ha hecho algo",
  "es tu escena — ¿qué hay en el escenario?".

Ese último punto tiene nombre: **inversión de control**. Normalmente *tu* código llama a
una librería. Aquí el engine es dueño del bucle y llama a *tu* código en los momentos
adecuados — "no nos llames, ya te llamamos nosotros". `winit` ya funciona así contigo (el
`ApplicationHandler` de § 2); ahora tu engine hace lo mismo para scop.

El guion habla con la compañía mediante un acuerdo escrito y breve: "puedes pedir un
objeto de atrezo (una mesh), un montaje de luces (un pipeline), y en cada escena puedes
decir qué hay en el escenario". Ese acuerdo es la **API pública**. Todo lo demás —
swapchains, fences, tipos de memoria — se queda entre bastidores, invisible para la obra.
Si la compañía rehace su tramoya (una versión nueva de `winit`, render passes → dynamic
rendering), las obras no cambian.

**Dónde deja de valer la analogía:** una compañía de teatro y una obra son negocios
distintos que pueden discutir el contrato. Aquí escribes tú los dos lados — así que tienta
"meter la mano entre bastidores" cuando a la API le falta algo. Cada vez que lo haces, el
siguiente proyecto hereda ese atajo.

---

## Los nombres

**public API** — todo lo que un crate marca como `pub` y es accesible desde fuera: los
tipos, funciones y traits que otros crates pueden usar. **API** = **A**pplication
**P**rogramming **I**nterface (§ 2): la superficie acordada entre dos piezas de código.

**crate boundary** — la línea entre dos crates. Rust la hace cumplir: scop solo puede tocar
lo que el engine marca como `pub`. `pub(crate)` (§ 5) significa "público solo dentro de
este crate" — entre bastidores.

**coupling** — cuánto depende una pieza de código de los detalles de otra. Acoplamiento
*fuerte*: scop importa tipos de `ash` y `winit`, así que cambiar las tripas del engine
rompe scop. Acoplamiento *débil*: scop solo nombra tipos del engine.

**inversion of control** — el framework es dueño del bucle principal y llama a tu código,
en vez de que tu código llame al framework. También llamado "el principio de Hollywood".

**App trait** — el contrato que implementa scop: un `trait` de Rust (§ 1) con los métodos
que llamará el engine — preparar, reaccionar a la entrada, describir el fotograma.

**default method** — un método de trait con cuerpo: `fn update(&mut self, …) {}`. Quien lo
implementa puede sobrescribirlo o dejarlo. Permite a una app implementar solo lo que
necesita.

**generic function** — `fn run<A: App>(app: A)` (o `fn run(app: impl App)`): una función
que sirve para cualquier tipo que implemente `App`. El compilador hace una copia
especializada para cada tipo que se use de verdad — sin coste en tiempo de ejecución.

**newtype** — un struct que envuelve un valor para darle su propio tipo:
`pub struct MeshId(usize)`. Un `MeshId` y un `PipelineId` son ambos números por dentro,
pero el compilador no te deja pasar uno donde se espera el otro.

**typed handle** — un id newtype que el engine le da a la app en vez del objeto en sí.
"Aquí tienes el resguardo `MeshId(0)`; la mesh se queda en el guardarropa." El engine
conserva el ownership, así que la app no puede destruir una mesh mientras la GPU todavía
la está dibujando.

**draw list** — lo que la app le entrega al engine cada fotograma: "limpia a este color;
dibuja la mesh 0 con el pipeline 0". Una lista de *peticiones*, que el engine graba en el
command buffer.

**input abstraction** — el propio enum `Key` del engine, traducido desde los códigos de
tecla de `winit`. scop dice `Key::Space`, nunca `winit::keyboard::KeyCode::Space`.

**delta time** — segundos desde el fotograma anterior, que se pasan a `update`. Un
movimiento escalado por él va a la misma velocidad a 30 o a 120 fotogramas por segundo (la
rotación de § 13).

**re-export** — `pub use crate::mesh::Vertex;` en `lib.rs`: hace que un elemento esté
disponible como `engine::Vertex`, para que la API pública sea una lista ordenada en vez de
la estructura interna de módulos del engine.

---

## Cómo funciona realmente

### 1. Clasificar qué va dónde

| Engine (todos los proyectos) | scop (solo este proyecto) |
|---|---|
| Ventana, bucle de eventos, traducción de teclas | Qué teclas hacen qué (§ 13, § 15) |
| Configuración de Vulkan, swapchain, fotogramas, sincronización | El color de limpieza |
| Pipelines a partir de SPIR-V, el cargador de SPIR-V | Los propios shaders GLSL |
| Buffers, subidas, meshes | ¿El parser de `.obj`? — **engine** (ft_newton también carga modelos), § 12 |
| Disposición del vértice | La ruta del modelo, los controles, la elección de textura |
| Draw list → command buffer | Qué dibujar cada fotograma |

Una prueba para cualquier cosa nueva: *¿la necesitaría ft_newton, sin cambios?* Sí →
engine. No → scop.

### 2. La forma de la API

```rust
// engine/src/lib.rs — toda la superficie pública, reexportada
pub use app::{App, Flow, Frame, Gfx};
pub use input::{Input, Key};
pub use mesh::{MeshData, Vertex};
pub use pipeline::Shaders;
pub use renderer::{MeshId, PipelineId};
pub use platform::{run, WindowConfig, RunError};
```

```rust
pub trait App {
    /// Una vez, cuando ya existen la ventana y la GPU: crear pipelines y meshes.
    fn setup(&mut self, gfx: &mut Gfx) -> Result<(), RunError>;
    /// Cada fotograma, antes de dibujar. Devuelve Flow::Exit para salir.
    fn update(&mut self, _input: &Input, _dt: f32) -> Flow { Flow::Continue }
    /// Cada fotograma: decir qué dibujar.
    fn draw(&self, frame: &mut Frame);
}
```

- **`Gfx`** — la ventana de la app a la GPU durante el setup: `create_pipeline(Shaders) ->
  PipelineId`, `create_mesh(&MeshData) -> MeshId`, más información de solo lectura (nombre
  de la GPU, formato de la swapchain) para imprimir.
- **`Frame`** — la draw list de un fotograma: `clear(colour)`, `draw(pipeline, mesh)`.
- **`Input`** — qué `Key`s están mantenidas y cuáles se pulsaron en este fotograma.
- **`Flow`** — `Continue` o `Exit`. Esc sale del engine y pasa a scop: salir es una
  decisión del guion.

### 3. Dentro del engine

`GpuContext` deja de poseer "el" pipeline y "la" mesh. Un `Renderer` (campos privados del
engine) posee un `Vec<Pipeline>` y un `Vec<Mesh>`; `PipelineId(i)` y `MeshId(i)` son
índices dentro de ellos. `draw_frame` recibe la draw list del fotograma y graba cada
petición: bind del pipeline, bind de los buffers, `draw_indexed`. Todo se sigue destruyendo
en `Drop`, después de `device_wait_idle` — la app nunca destruye objetos de GPU por su
cuenta.

### 4. Dentro de scop

```rust
struct Scop { pipeline: Option<PipelineId>, quad: Option<MeshId>, start: Instant }

impl App for Scop {
    fn setup(&mut self, gfx: &mut Gfx) -> Result<(), RunError> { … crear los dos, imprimir info de GPU … }
    fn update(&mut self, input: &Input, _dt: f32) -> Flow {
        if input.pressed(Key::Escape) { Flow::Exit } else { Flow::Continue }
    }
    fn draw(&self, frame: &mut Frame) { frame.clear(pulsing_blue(…)); frame.draw(…); }
}
```

`main` pasa a ser: leer argumentos, construir `Scop`, `engine::run(config, scop)`. **El
`Cargo.toml` de scop ya no necesita `ash` ni `winit`, y su código no nombra ninguno.**

### Decisión: cómo se conecta scop

| | **A. trait `App`** (recomendado) | B. Struct de callbacks | C. La app es dueña del bucle |
|---|---|---|---|
| Forma | `impl App for Scop` | `run(Callbacks { setup: …, draw: … })` | `loop { engine.poll(); engine.draw(); }` |
| Estado de la app | Vive en `self`, de forma natural | Capturado en closures o globales — incómodo de compartir entre callbacks | Variables locales normales |
| Encaja con `winit` 0.30 | Sí — mismo estilo de callbacks | Sí | Mal — `winit` 0.30 quiere ser dueño del bucle |
| Crece hacia ft_newton | Añades un método por defecto; las apps viejas siguen compilando | Añades un campo; todas las apps deben cambiar | Cada app reescribe el bucle |
| Coste | Un trait cuya forma hay que aprender | Muchos tipos de función | Pelearse con `winit` |

Recomendación: **A**. Confírmalo o cámbialo en el gate.

### 5. Lo que se queda pequeño a propósito

Sin grafo de escena, sin sistema de entidades, sin gestor de assets, sin recarga en
caliente. Una ventana, unos pocos pipelines, unas pocas meshes. La regla del plan (§ 00):
el engine crece cuando un proyecto lo necesita, no antes.

---

## Trampas conocidas

- **La abstracción con fugas** — un campo `pub` de tipo `vk::Buffer` o `winit::KeyCode`
  en la API pública. scop vuelve a depender de `ash`/`winit`, solo que de forma indirecta.
  Comprueba: el `Cargo.toml` de scop solo tiene `engine`; `cargo build` fallaría con
  cualquier `ash::` en scop.
- **Ids de otro engine o fuera de rango** — `MeshId(7)` cuando hay 2 meshes. Mantén el
  número privado — `pub struct MeshId(usize)`, no `pub struct MeshId(pub usize)` — para
  que solo el engine pueda crear uno.
- **La app guardando objetos de GPU** — si `create_mesh` devolviera una `Mesh` propiedad
  de scop, scop podría soltarla a mitad de fotograma: use-after-free en la GPU (el "in use"
  de § 5). Los handles lo impiden.
- **Sobreingeniería** — diseñar ahora para las necesidades de ft_minecraft. Construye solo
  lo que usa scop; deja *sitios* claros para crecer (métodos por defecto, nuevas peticiones
  en `Frame`).
- **Romper el orden de destrucción** — la refactorización mueve objetos de sitio;
  `Graphics` debe seguir soltando la GPU antes que la ventana, y las meshes/pipelines antes
  que el dispositivo.
- **Un cambio de comportamiento escondido en una refactorización** — una refactorización
  debería verse idéntica en pantalla. Mismo quad, mismo pulso, Esc sigue saliendo, ningún
  mensaje de validación. Compruébalo antes y después.

---

## Checkpoint

**Cumple:** ningún cambio visible — de eso se trata. Prepara § 12–15, donde scop añade
controles y modelos sin tocar las tripas del engine.
(Método: primero el gate, luego Claude lo escribe y te lo explica.)

1. Engine: trait `App`, `Gfx`, `Frame` (draw list), `Flow`, `Input` + `Key`, newtypes
   `MeshId` / `PipelineId` con campos privados, `run(config, app)`.
2. Engine: un renderer que posee pipelines y meshes en `Vec`s; `draw_frame` graba la draw
   list del fotograma.
3. Engine: una lista ordenada de re-exports en `lib.rs`; las tripas `pub(crate)` o
   privadas.
4. scop: `struct Scop` implementando `App`; Esc gestionado en scop con `Flow::Exit`.
5. El `Cargo.toml` de scop depende solo de `engine`; ni `ash`, ni `winit`, ni `vk::` en
   ninguna parte de scop.
6. **Demuestra la frontera:** prueba `use engine::buffer::Buffer;` en scop — debe fallar al
   compilar (privado). Quítalo.

**Verificar:** comportamiento idéntico a § 7 (fondo que pulsa, quad de cuatro colores, Esc
sale, ningún mensaje de validación); `cargo test` y `cargo clippy` limpios.

---

## Preguntas

Q-8.1 (recuerdo) ¿Qué significa *inversión de control* en este engine: quién es dueño del
bucle principal, y quién llama a quién?

Q-8.2 (aplicación) Clasifica en engine o scop: (a) la ruta del archivo `.obj` a mostrar;
(b) la swapchain; (c) el color del fondo; (d) el cargador de SPIR-V; (e) el código GLSL de
los shaders de scop; (f) traducir los códigos de tecla de `winit`; (g) qué tecla activa o
desactiva la textura.

Q-8.3 (predicción) Supón que `Gfx::create_mesh` devolviera la propia `Mesh` (con sus
buffers) en vez de un `MeshId`, y que scop la soltara mientras hay fotogramas en vuelo. ¿Qué
pasa, y cómo lo impide un handle?

Q-8.4 (predicción) `App::update` recibe el `KeyCode` propio de `winit` en vez del `Key` del
engine. Un año después actualizas `winit` y renombra algunas teclas. ¿Qué tiene que cambiar
en ft_newton, ft_vox y scop — y qué tendría que cambiar con el `Key` del engine?

Q-8.5 (aplicación) ¿Cuáles de estos deberían ser `pub` en el engine, y cuáles `pub(crate)`
o privados: `Vertex`, `Buffer`, `MeshId`, `find_memory_type`, `App`,
`GpuContext::draw_frame`? Un motivo para cada uno.

Respuestas: `ANSWERS.md` § 8
