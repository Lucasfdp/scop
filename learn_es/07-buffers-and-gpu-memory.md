---
section: 7
slug: buffers-and-gpu-memory
tier: T2
status: done
prereqs: [6]
terms: [mesh, buffer, buffer usage, device memory, memory requirements, memory type, memory heap, DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT, map, bind (memory), staging buffer, cmd_copy_buffer, vertex buffer, index buffer, index type, stride, vertex binding, vertex attribute, attribute format, offset_of!, draw_indexed]
---

# Sección 7 — Enviar datos a la GPU

**Profundidad: T2** — todo modelo que scop muestre, del logo de 42 a la tetera, llega a la
GPU exactamente por este camino. Y las texturas de § 14 reutilizan el mismo código de
subida. Si sabes explicar adónde va cada byte y por qué, el parser de `.obj` de § 12 es
solo "producir los bytes".
**Serás capaz de:** poner datos de vértices e índices en la memoria de la GPU, elegir el
tipo de memoria adecuado, describir su disposición al pipeline y dibujar una forma hecha
de esquinas compartidas.

---

## La idea

Las esquinas de tu triángulo viven *dentro del vertex shader*. Eso funciona para tres
puntos escritos a mano. No puede funcionar para una tetera: sus esquinas vienen de un
archivo, en tiempo de ejecución, miles de ellas. Tienen que viajar desde tu programa a
memoria que la GPU pueda leer, y hay que decirle al pipeline qué aspecto tienen.

Volvamos a la fábrica de § 6. El puesto 1 se sabía sus tres esquinas de memoria. Ahora
recibe una **caja de piezas** — los datos de las esquinas — más un **albarán** que dice
"cada pieza ocupa 20 bytes: los 8 primeros son una posición, los 12 siguientes un color".
La caja es un **vertex buffer**; el albarán es la **descripción de vertex input** del
pipeline.

Segunda idea. La mayoría de esquinas de un modelo 3D se comparten: un cuadrado son dos
triángulos, pero solo tiene *cuatro* esquinas, no seis. En vez de enviar seis esquinas
(dos de ellas copias), envías las cuatro una vez, más una lista corta de **instrucciones
de montaje** — "el triángulo 1 usa las esquinas 0, 1, 2; el triángulo 2 usa 2, 3, 0". Esa
lista es un **index buffer**. En un modelo real, donde cada esquina la comparten unos seis
triángulos, esto ahorra la mayor parte de los datos.

Tercera idea: la GPU tiene **distintos tipos de memoria**. Una es rápida para la GPU pero
la CPU no puede tocarla directamente — el **almacén** de la fábrica. En otra la CPU puede
escribir directamente — el **muelle de carga**. (En tu M2 las dos son físicamente los
mismos chips — "memoria unificada" — pero Vulkan las describe por separado igualmente, y en
un PC del campus con tarjeta gráfica sí son chips distintos.) La forma habitual de
entregar: la CPU deja la mercancía en el muelle de carga y luego la carretilla de la GPU la
lleva al almacén. El buffer del muelle es un **staging buffer**; la carretilla es un
**comando de copia**.

**Dónde deja de valer la analogía:** un buffer es solo un rango de bytes sin significado
propio. "Esto es una posición" solo existe en el albarán. Si el albarán dice algo
equivocado, la GPU lee los bytes mal sin quejarse — obtienes una forma basura, no un error.

---

## Los nombres

**mesh** — una forma descrita como **esquinas más triángulos**: una lista de vértices y una
lista de índices que dice qué tres esquinas forman cada triángulo. La palabra significa
*malla* (red): los triángulos forman una red tensada sobre la superficie de la forma. Todo
modelo 3D de scop — el logo de 42, la tetera — es una mesh. En el engine, `MeshData` es la
mesh en la CPU (dos `Vec`), `Mesh` es la misma mesh en la memoria de la GPU (dos buffers).

**buffer** — `VkBuffer`: un handle que describe "una tira de N bytes que la GPU puede usar
para estos fines". Por sí solo **no tiene memoria**: es una etiqueta esperando un sitio.

**buffer usage** — para qué puede usarse el buffer, declarado de antemano:
`VERTEX_BUFFER`, `INDEX_BUFFER`, `TRANSFER_SRC` (copiar *desde* él), `TRANSFER_DST`
(copiar *hacia* él), más adelante `UNIFORM_BUFFER` (§ 10). Usarlo para algo no declarado
no es válido.

**device memory** — `VkDeviceMemory`: un bloque real de memoria que *reservas* (allocate),
al que luego enganchas buffers. Dos objetos en vez de uno porque en el mundo de las GPUs se
separa "lo que necesito" de "dónde vive": los engines grandes meten muchos buffers en una
sola reserva.

**memory requirements** — lo que le preguntas a un buffer tras crearlo: cuántos bytes
necesita de verdad (puede ser más de lo que pediste), la alineación, y
**`memory_type_bits`** — una máscara de bits de en qué tipos de memoria puede vivir (bit
*i* activado = tipo *i* permitido).

**memory type / memory heap** — la GPU lista sus tipos de memoria como *heaps* (bolsas
físicas, p. ej. "los 8 GB de la tarjeta gráfica") y *types* (formas de usar un heap, cada
una con flags de propiedades). Eliges un tipo por índice.

**`DEVICE_LOCAL`** — la memoria es rápida para la GPU (el almacén).

**`HOST_VISIBLE`** — la CPU (*host*, en el vocabulario de Vulkan) puede mapearla y escribir
en ella (el muelle de carga).

**`HOST_COHERENT`** — las escrituras de la CPU se hacen visibles a la GPU automáticamente.
Sin ello tienes que llamar a `flush_mapped_memory_ranges` tras escribir — fácil de olvidar,
así que scop siempre pide memoria coherente cuando escribe desde la CPU.

**map** — `map_memory`: obtener un puntero de la CPU (`*mut c_void`) a memoria
host-visible. Escribes a través de él como con cualquier puntero de C, y luego
`unmap_memory`.

**bind (memory)** — `bind_buffer_memory(buffer, memory, offset)`: enganchar un buffer a
(una parte de) una reserva. Una sola vez por buffer; no se puede cambiar.

**staging buffer** — un buffer host-visible temporal que solo se usa para llevar datos a
un buffer device-local. Se crea, se llena, se copia desde él y se destruye.

**`cmd_copy_buffer`** — un comando de transferencia grabado: copiar bytes de un buffer a
otro. Necesita `TRANSFER_SRC` en el origen y `TRANSFER_DST` en el destino.

**vertex buffer / index buffer** — buffers cuyo uso es `VERTEX_BUFFER` / `INDEX_BUFFER`:
los datos de las esquinas, y la lista de números de esquina que forman triángulos.

**index type** — cuánto ocupa cada índice: `UINT16` (hasta 65 535 esquinas) o `UINT32`
(unos 4000 millones). scop usa `UINT32` — un `.obj` detallado puede pasar de 65 535
esquinas.

**stride** — cuántos bytes hay del inicio de un vértice al inicio del siguiente. Para el
vértice de scop ahora: 20.

**vertex binding** — `VertexInputBindingDescription`: "el hueco de binding 0 lee un buffer
cuyos vértices están separados `stride` bytes, un vértice por esquina" (`input rate
VERTEX`).

**vertex attribute** — `VertexInputAttributeDescription`: "el valor para la `location` N
del shader está en el byte `offset` dentro de cada vértice, en formato F".

**attribute format** — los formatos de vertex input reutilizan los nombres de formatos de
imagen: `R32G32_SFLOAT` = **dos** **float**s con **s**igno de 32 bits (un `vec2`),
`R32G32B32_SFLOAT` = tres (un `vec3`). *R, G, B* aquí solo significan "1ª, 2ª, 3ª
componente", no colores.

**`offset_of!`** — una macro de Rust que da el desplazamiento en bytes de un campo dentro
de un struct (`offset_of!(Vertex, colour)` = 8). Con `#[repr(C)]` (§ 1) la disposición es
la de C — campos en el orden escrito, relleno predecible — así que los números son estables
y coinciden con lo que calcularías a mano.

**`draw_indexed`** — `cmd_draw_indexed(index_count, instances, first_index,
vertex_offset, first_instance)`: dibujar recorriendo el index buffer en vez de los vértices
en orden.

---

## Cómo funciona realmente

### El camino de un vértice

```
Rust: [Vertex; 4]  ──map + copia──►  staging buffer (HOST_VISIBLE | HOST_COHERENT)
                                         │  cmd_copy_buffer (command buffer de un solo uso,
                                         ▼   submit, esperar)
                                    vertex buffer (DEVICE_LOCAL)
                                         │  bind_vertex_buffers(binding 0)
                                         ▼
vertex input del pipeline: stride 20, location 0 = 2 floats @0, location 1 = 3 floats @8
                                         ▼
vertex shader: layout(location = 0) in vec2 in_position; layout(location = 1) in vec3 in_colour;
```

### 1. El vértice en Rust

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 2], // bytes 0..8
    pub colour: [f32; 3],   // bytes 8..20
}
```

`Clone, Copy` es lo correcto aquí (a diferencia de `GpuContext` en § 3): un vértice son
números sueltos, no el dueño de nada.

### 2. Crear un buffer con memoria

1. `create_buffer`: tamaño, uso, compartición `EXCLUSIVE`.
2. `get_buffer_memory_requirements` → tamaño, alineación, `memory_type_bits`.
3. **Buscar un tipo de memoria**: para cada tipo *i* que lista la GPU, quédate con el
   primero cuyo bit *i* de `memory_type_bits` esté activado **y** cuyos flags de
   propiedades contengan los que quieres.
4. `allocate_memory` con el tamaño *requerido* (no el tamaño de tus datos) y ese índice
   de tipo.
5. `bind_buffer_memory(buffer, memory, 0)`.

Se destruye al revés: `destroy_buffer`, luego `free_memory`.

### 3. Subir a través de un staging buffer

1. Staging buffer: uso `TRANSFER_SRC`, memoria `HOST_VISIBLE | HOST_COHERENT`.
2. `map_memory` → puntero; copia dentro los bytes de los vértices (`copy_nonoverlapping`,
   el `memcpy` de Rust); `unmap_memory`.
3. Buffer final: uso `VERTEX_BUFFER | TRANSFER_DST`, memoria `DEVICE_LOCAL`.
4. Un **command buffer de un solo uso** del command pool: begin (`ONE_TIME_SUBMIT`),
   `cmd_copy_buffer`, end, submit con una fence, **esperar** a esa fence.
5. Destruir el staging buffer y liberar su memoria — solo *después* de la espera.

El index buffer sigue el mismo camino con `INDEX_BUFFER` en vez de `VERTEX_BUFFER`. Es
trabajo en tiempo de carga, una vez por modelo — la espera no toca el bucle de fotogramas.

### Decisión (tomada el 2026-11-08): staging → device-local para las meshes

| | Staging → device-local (recomendado) | Solo host-visible |
|---|---|---|
| Código | Un comando de copia, un submit de un solo uso y una espera | Mapear, copiar, listo |
| Velocidad de lectura de la GPU | La mejor en cualquier GPU | Bien en tu M2 (memoria unificada); más lento en las tarjetas gráficas del campus, donde la memoria host-visible está del lado de la CPU |
| Reutilización | **Las texturas de § 14 necesitan exactamente este camino** (las imágenes solo se pueden llenar con una copia) | Las texturas lo necesitarían igualmente |
| Cuándo encaja | Datos escritos una vez, leídos cada fotograma (meshes) | Datos reescritos cada fotograma (los uniforms de § 10) |

**Elegido: staging para las meshes** (registrado en `DECISIONS.md`); memoria host-visible
para datos que se reescriben cada fotograma (los uniforms de § 10). En tu M2 el único coste
es una copia extra en tiempo de carga — la velocidad de dibujo por fotograma es la misma.

*Idea aplazada* (`00-PLAN.md` → Aplazado / v2): cuando una GPU ofrece memoria que es a la
vez `DEVICE_LOCAL` y `HOST_VISIBLE` (memoria unificada, o Resizable BAR en tarjetas de PC),
un engine puede saltarse el staging y escribir directamente. Merece la pena en ft_vox, no
en scop.

### 4. Decírselo al pipeline

```
binding 0:   stride = size_of::<Vertex>() = 20, input rate VERTEX
location 0:  binding 0, formato R32G32_SFLOAT,    offset = offset_of!(Vertex, position) = 0
location 1:  binding 0, formato R32G32B32_SFLOAT, offset = offset_of!(Vertex, colour)   = 8
```

Esto va al vertex input state del pipeline, que en § 6 estaba vacío.

### 5. El shader

```glsl
layout(location = 0) in vec2 in_position;
layout(location = 1) in vec3 in_colour;
layout(location = 0) out vec3 colour;

void main() {
    gl_Position = vec4(in_position, 0.0, 1.0);
    colour = in_colour;
}
```

Las locations `in` (del vertex buffer) y las `out` (hacia el fragment shader) son
numeraciones separadas — las dos pueden tener un 0.

### 6. Dibujar

```
bind_pipeline
bind_vertex_buffers(primer binding 0, [vertex buffer], [offset 0])
bind_index_buffer(index buffer, offset 0, UINT32)
draw_indexed(index_count = 6, 1 instancia, primer índice 0, vertex offset 0, primera instancia 0)
```

### 7. Ownership

Un tipo `Buffer` en el engine posee un buffer **y** su memoria, y destruye los dos. Una
`Mesh` posee un `Buffer` de vértices, un `Buffer` de índices y el número de índices. Los
buffers de la mesh se crean a partir del dispositivo y se destruyen antes que él, después
de `device_wait_idle`. (Una reserva por buffer está bien para scop; los drivers permiten al
menos 4096 reservas. Un allocator de verdad que empaquete buffers en bloques grandes es
terreno de ft_vox.)

---

## Trampas conocidas

- **Forma basura, sin errores** — el albarán no encaja con los bytes: stride erróneo,
  offset erróneo, formato erróneo (`R32G32B32` para un `vec2`), o una `location` que no
  coincide con el shader.
- **Struct sin `#[repr(C)]`** — Rust puede reordenar los campos. Con `offset_of!` los
  números siguen siendo correctos, pero cualquier offset escrito a mano se rompe en
  silencio.
- **Error de validación: buffer no creado con `TRANSFER_DST`** (o `TRANSFER_SRC` en el
  staging buffer) — el uso debe nombrar todas las formas en que se usa.
- **Error de validación sobre el tipo de memoria** — elegiste un tipo sin comprobar
  `memory_type_bits`.
- **Reservar el tamaño de tus datos en vez del tamaño requerido** — puede que el driver
  necesite más (alineación, relleno). Reserva `requirements.size`.
- **Destruir el staging buffer antes de que se ejecute la copia** — el submit volvió
  enseguida; la copia ocurre después. Espera antes a la fence. La validación reporta "in
  use".
- **Falta la mitad de los triángulos, o salen pinchos** — tipo de índice que no coincide:
  datos `u16` declarados como `UINT32` (o al revés).
- **Solo el primer triángulo bien / forma revuelta** — llamaste a `draw` en vez de
  `draw_indexed`: la GPU recorre los vértices en orden e ignora el index buffer.
- **Al salir: "VkDeviceMemory has not been freed"** — destruiste el buffer pero olvidaste
  `free_memory`.

---

## Checkpoint

**Cumple:** "renderizarlo en una ventana" para datos que no están escritos a mano en un
shader — el camino que alimentará el cargador de `.obj` de § 12. (Método: primero el gate,
luego Claude lo escribe y te lo explica.)

1. Engine: un tipo `Vertex` (`#[repr(C)]`, posición + color) y su descripción de vertex
   input, construida con `size_of` / `offset_of!`.
2. Engine: un tipo `Buffer` (buffer + memoria, búsqueda de tipo de memoria, destruye los
   dos).
3. Engine: subida a través de un staging buffer con un command buffer de un solo uso.
4. Engine: una `Mesh` (vertex buffer, index buffer, número de índices) dibujada con
   `draw_indexed`.
5. scop: un **quad** de 4 esquinas de colores y 6 índices, en lugar del triángulo; el
   vertex shader lee atributos `in`.
6. **Demuestra que los índices importan:** cambia un índice, mira cómo cambia la forma del
   quad, vuelve a ponerlo.

**Verificar:** un quad de cuatro colores sobre el fondo que pulsa; ningún mensaje de
validación, tampoco al salir (toda la memoria liberada); `cargo clippy` limpio.

---

## Preguntas

Q-7.1 (recuerdo) ¿Qué diferencia hay entre un `VkBuffer` y un `VkDeviceMemory`? ¿Por qué
Vulkan los hace dos objetos separados?

Q-7.2 (aplicación) Un cuadrado hecho de dos triángulos: ¿cuántos vértices se guardan con un
index buffer, y cuántos índices? ¿Cuántos vértices sin él? ¿Por qué el ahorro importa mucho
más para una tetera que para un cuadrado?

Q-7.3 (predicción) Envías la copia de staging y destruyes el staging buffer en la línea
siguiente, sin esperar. ¿Qué podría pasar, y qué dice la validación?

Q-7.4 (aplicación) ¿Qué flags de propiedades de memoria y qué flags de uso necesitan (a) el
staging buffer y (b) el vertex buffer final, y por qué cada uno?

Q-7.5 (predicción) El atributo de vértice de `in_colour` se declara con offset 0 en vez de
8. Nada más cambia. ¿Qué ves?

Respuestas: `ANSWERS.md` § 7
