---
section: 14
slug: textures
tier: T2
status: done
prereqs: [13]
terms: [texture, texel, texture coordinates, planar mapping, stretching, TGA, image header, BGR order, image origin, run-length encoding, sRGB, texture image, layout transition for upload, copy buffer to image, sampler, filtering, address mode, combined image sampler, descriptor set per texture, default white texture, mipmaps]
---

# Sección 14 — Imágenes sobre superficies

**Profundidad: T2** — las texturas son el segundo tipo de descriptor (tras el uniform
buffer de § 10) y la primera *imagen* de GPU que llenas con tus propios datos.
Reutilizarás cada pieza de esto en ft_vox (texturas de bloques) y más allá. Deberías saber
explicar el camino de un archivo en disco a un píxel coloreado, y diagnosticar los fallos
clásicos (boca abajo, desvaído, emborronado) a partir de una captura.
**Serás capaz de:** cargar una imagen `.tga` con tu propio cargador, ponerla sobre el logo
de 42 y decir por qué está cada paso.

---

## La idea

Hasta ahora el color de cada píxel venía de sus esquinas: el rasterizador mezclaba los
colores de las esquinas a lo largo del triángulo (§ 6, § 13). Una textura funciona
distinto: las esquinas dicen **en qué punto de una imagen** están, y cada píxel busca su
color en la imagen.

Imagina envolver un regalo. El papel de regalo es la imagen. Para cada esquina de la caja,
apuntas *qué punto del papel* acaba ahí: "esta esquina está en la esquina superior
izquierda del papel", "esta al 30 % de ancho, 70 % de alto". Esas notas son solo dos números
por esquina, de 0 a 1, en horizontal y en vertical. La GPU mezcla *esos* números a lo largo
de cada triángulo — exactamente como mezclaba los colores — así que cada píxel sabe su propio
punto del papel, y lee el color de ahí.

Tienen que pasar cuatro cosas:

1. **Leer el archivo de imagen a memoria** — tu propio cargador, porque el subject prohíbe
   las librerías. scop usa el formato `.tga`: el formato común más sencillo que las
   herramientas de imagen de verdad todavía guardan.
2. **Dar a cada esquina su posición en el papel.** Los archivos `.obj` no traen ninguna, así
   que scop se las inventa: mirar el modelo desde el lado donde es más ancho, y poner la
   imagen plana encima, como un proyector que la atraviesa en línea recta. Sencillo — pero
   las superficies que van *a lo largo* del haz del proyector quedan estiradas en rayas.
   Arreglar eso es el bonus de § 18.
3. **Copiar la imagen a la memoria de la GPU** — el mismo viaje de staging que las meshes
   (§ 7), pero a una *imagen* de GPU en vez de a un buffer.
4. **Dejar que el fragment shader la lea** — mediante un descriptor, como la cámara de
   § 10, más un pequeño objeto de ajustes (el *sampler*) que dice cómo leer entre píxeles y
   qué hacer más allá de los bordes.

**Dónde deja de valer la analogía:** el papel de regalo es una hoja continua. Una textura es
una cuadrícula de cuadraditos de color separados, así que "el color al 30 % de ancho" suele
caer *entre* cuadraditos — el sampler decide si coger el más cercano o mezclar los vecinos.

---

## Los nombres

**texture** — una imagen usada para colorear superficies. En Vulkan: un `VkImage` + view +
sampler, alcanzado a través de un descriptor.

**texel** — un cuadradito de color de una textura: **tex**ture **el**ement, como *pixel* es
**pic**ture **el**ement. Se distingue para que "píxel" pueda seguir significando "en
pantalla".

**texture coordinates** — la posición de una esquina en la imagen: dos números, normalmente
llamados **U** (horizontal, 0 = izquierda, 1 = derecha) y **V** (vertical, 0 = arriba, 1 =
abajo). A menudo solo "UVs" — esas letras estaban libres porque x, y, z ya estaban cogidas.

**planar mapping** — inventar las UVs proyectando la imagen en línea recta a través del
modelo a lo largo de un eje: quitar la coordenada de ese eje y escalar las otras dos a 0..1.

**stretching** — en superficies paralelas a la proyección, mucha superficie comparte la
misma franja de UV, así que los píxeles del borde de la imagen se emborronan a lo largo de
ellas. El defecto visible que el bonus de § 18 te pide arreglar.

**TGA** — **T**ruevision **G**raphics **A**dapter (también "Targa"), un formato de imagen
de 1984: una cabecera de 18 bytes y luego los píxeles. Fácil de leer a mano, y todavía lo
exportan GIMP, Photoshop y `sips` de macOS.

**image header** — los bytes fijos al principio del archivo que describen el resto: tipo de
imagen, ancho, alto, bits por píxel, y un byte "descriptor" con el flag de origen.

**BGR order** — TGA guarda cada píxel como **B**lue (azul), **G**reen (verde), **R**ed
(rojo) (y **A**lpha si es de 32 bits): al revés que RGB. El cargador los intercambia.

**image origin** — si la primera fila guardada es la de **abajo** de la imagen (lo que hace
TGA por defecto) o la de **arriba** (bit 5 del byte descriptor activado). Ignóralo y la
imagen sale boca abajo.

**run-length encoding (RLE)** — compresión sencilla: "los 37 píxeles siguientes son todos
de este color" guardado una vez en vez de 37. TGA tipo de imagen 10; GIMP suele guardar
así.

**sRGB** — la forma estándar en que los archivos de imagen guardan el brillo: más números
para los tonos oscuros que para los claros, porque el ojo es más sensible ahí. Hay que
decirle a la GPU que una textura es sRGB (`R8G8B8A8_SRGB`) para que la convierta de vuelta a
brillo real antes de que la use el shader, y tu swapchain (también sRGB, § 4) vuelve a
convertir a la salida. Hazlo mal y los colores se ven desvaídos o demasiado oscuros.

**texture image** — un `VkImage` con uso `TRANSFER_DST` (una copia va a escribir en él) y
`SAMPLED` (los shaders lo leerán), en memoria `DEVICE_LOCAL`, tiling `OPTIMAL`.

**layout transition for upload** — dos barreras alrededor de la copia:
`UNDEFINED → TRANSFER_DST_OPTIMAL` ("lista para que la escriba una copia"), y luego
`TRANSFER_DST_OPTIMAL → SHADER_READ_ONLY_OPTIMAL` ("lista para que la lean los shaders").
La misma idea que la imagen de color de § 5, con otras etapas.

**copy buffer to image** — `cmd_copy_buffer_to_image`: los bytes del staging buffer a los
texels de la imagen, fila a fila.

**sampler** — `VkSampler`: las reglas de lectura de una textura. Separado de la imagen, para
que una imagen se pueda leer de formas distintas.

**filtering** — qué hacer cuando una consulta cae entre texels: `NEAREST` (coger el más
cercano: pixelado de cerca) o `LINEAR` (mezclar los cuatro de alrededor: suave). scop usa
`LINEAR`.

**address mode** — qué hacer con UVs fuera de 0..1: `REPEAT` (repetir la imagen en
mosaico), `CLAMP_TO_EDGE` (estirar los texels del borde). scop usa `REPEAT`.

**combined image sampler** — un tipo de descriptor que apunta a una image view *y* a un
sampler a la vez. En GLSL: `sampler2D`.

**descriptor set per texture** — cada textura tiene su propio pequeño descriptor set (set 1
en el shader), escrito una vez; un draw enlaza el set de la textura que quiere. La misma
idea de puntero que en § 10: "qué imagen" se elige por qué set está enlazado.

**default white texture** — una textura blanca de 1×1 que el engine siempre tiene, para que
todo draw tenga *alguna* textura enlazada. Multiplicar por blanco no cambia nada.

**mipmaps** — copias más pequeñas, ya reducidas, de una textura que se usan cuando está
lejos, para evitar el centelleo. scop se las salta; menciónalas si te preguntan en la
defensa.

---

## Cómo funciona realmente

### 1. El cargador de TGA (engine)

```
leer la cabecera de 18 bytes:
    byte 2       tipo de imagen   2 = true-colour sin comprimir, 10 = true-colour RLE
    bytes 12–13  ancho            (u16 little-endian)
    bytes 14–15  alto
    byte 16      bits por píxel   24 (BGR) o 32 (BGRA)
    byte 17      descriptor       bit 5: 1 = la primera fila es la de arriba
saltar el image ID (longitud en el byte 0) y cualquier mapa de colores (scop rechaza esos tipos)
leer ancho × alto píxeles:
    tipo 2:  solo los bytes
    tipo 10: paquetes — byte de cabecera h: si h & 0x80 → un píxel repetido (h & 0x7F) + 1 veces
                                            si no     → siguen (h & 0x7F) + 1 píxeles en crudo
intercambiar BGR(A) → RGBA (alpha 255 si es de 24 bits)
si el origen es abajo: invertir las filas para que la fila 0 sea la de arriba
```

Salida: `Image { width, height, rgba: Vec<u8> }`. Toda comprobación que puede fallar con un
archivo roto (demasiado corto, tipo no soportado, tamaño cero, píxeles truncados) se
convierte en un `TgaError` — nunca un panic, la misma regla que el parser de `.obj`.

### 2. UVs (scop)

```
buscar el eje más delgado de la bounding box → proyectar a lo largo de él
los otros dos ejes → (u, v), cada uno escalado a 0..1 según la bounding box
v se invierte (1 − v) para que la parte de arriba de la imagen quede arriba del modelo
```

Para el logo de 42 el eje más delgado es x, así que la imagen se pone sobre la cara y-z del
logo: el frente y la parte de atrás reciben la imagen, los lados delgados reciben rayas. El
`Vertex` gana `uv: [f32; 2]` (location 2, formato `R32G32_SFLOAT`).

### 3. Subida (engine)

```
staging buffer (HOST_VISIBLE) ← bytes RGBA
imagen: R8G8B8A8_SRGB, OPTIMAL, uso TRANSFER_DST | SAMPLED, memoria DEVICE_LOCAL
command buffer de un solo uso (el Uploader de § 7):
    barrera   UNDEFINED → TRANSFER_DST_OPTIMAL
    cmd_copy_buffer_to_image
    barrera   TRANSFER_DST_OPTIMAL → SHADER_READ_ONLY_OPTIMAL   (visible para lecturas de FRAGMENT_SHADER)
submit, esperar a su fence, destruir el staging buffer
view (aspect COLOR) + descriptor set (set 1, binding 0) apuntando a la view + el sampler compartido
```

### 4. Shaders

```glsl
// mesh.vert: dejar pasar la UV
layout(location = 2) in vec2 in_uv;
layout(location = 1) out vec2 uv;

// mesh.frag
layout(set = 1, binding = 0) uniform sampler2D picture;
layout(location = 1) in vec2 uv;
void main() { out_colour = texture(picture, uv); }
```

`texture(picture, uv)` es la consulta: "el color en (u, v), con las reglas del sampler". En
§ 15 el fragment shader mezclará esto con el gris; en este checkpoint muestra solo la
textura.

### 5. API del engine

```rust
let image = engine::load_tga(&bytes)?;          // tu cargador
let texture = gfx.create_texture(&image)?;      // → TextureId, como MeshId
frame.draw_textured(pipeline, mesh, texture, model);
frame.draw(pipeline, mesh, model);              // = textura blanca por defecto
```

### Decisión: TGA, set 1 por textura, UVs planas

| Elección | Frente a | Por qué |
|---|---|---|
| **TGA** (tipos 2 + 10) | PPM (menos herramientas lo exportan), BMP (reglas de relleno, varias versiones de cabecera), PNG (necesita un descompresor escrito a mano — un proyecto en sí mismo) | El formato más sencillo que las herramientas de verdad aún guardan; se prueba en la defensa con tu propio archivo |
| **Set 1 por textura** + blanca por defecto | Textura en el set 0 junto a la cámara | La cámara cambia por slot de fotograma, la textura por draw: ritmos de actualización distintos → sets distintos (otra vez la regla de § 10) |
| **UVs planas a partir de la bounding box** | Leer las líneas `vt` | Los archivos del subject no tienen; un mapeo sin estiramiento es el bonus de § 18 |
| **Textura por defecto incrustada** (`include_bytes!`) | Ruta en tiempo de ejecución | Funciona desde cualquier carpeta en la defensa; el mismo razonamiento que con los shaders (§ 6) |

---

## Trampas conocidas

- **Imagen boca abajo** — se ignoró el bit de origen (o se aplicó dos veces la inversión
  `1 − v`).
- **Colores intercambiados (un cielo azul se ve naranja)** — no se pasó de BGR a RGB.
- **Colores desvaídos / demasiado pálidos** — la imagen se creó como `R8G8B8A8_UNORM`
  mientras los bytes del archivo son sRGB (o al revés: demasiado oscuro). Usa
  `R8G8B8A8_SRGB`. (Q-14.5)
- **Basura en diagonal / rayas inclinadas** — el ancho o los bytes por píxel están mal por
  uno: cada fila empieza unos bytes desplazada, así que la imagen se cizalla.
- **Rayas largas en algunas caras** — mapeo plano en caras paralelas a la proyección.
  Esperado; bonus de § 18. (Q-14.2)
- **Error de validación: layout de imagen** — falta una barrera, o el descriptor se escribió
  con un layout distinto de `SHADER_READ_ONLY_OPTIMAL`.
- **Error de validación: descriptor set 1 no enlazado / no compatible** — el pipeline layout
  no declara el set 1, o el draw no enlazó un set de textura (eso es lo que evita la
  textura blanca por defecto).
- **Panic con un `.tga` raro** — un `unwrap` o un slice sin comprobar. Todo fallo debe ser
  un `TgaError`.

---

## Checkpoint

**Cumple (subject):** "aplicar una textura al objeto" (la tecla y el fundido suave son
§ 15). Método: primero el gate, luego Claude lo escribe y te lo explica.

0. **Tú:** elige una imagen alegre (gatito, poni, unicornio — tú eliges, una que puedas
   usar) y guárdala como `resources/texture.tga`. En macOS:
   `sips -s format tga picture.jpg --out resources/texture.tga`. Hasta entonces, un patrón
   generado hace de sustituto.
1. **Engine:** `engine/src/tga.rs` — `load_tga`, `Image`, `TgaError`; tests para 24/32
   bits, los dos orígenes, RLE y archivos rotos.
2. **Engine:** texturas — imagen + memoria + view, subida por staging con las dos
   barreras, un sampler compartido, un descriptor set por textura (set 1), la textura
   blanca por defecto, `Gfx::create_texture`, `Frame::draw_textured`, todo destruido en
   orden.
3. **Engine:** `Vertex::uv`, atributo en location 2; shaders actualizados.
4. **scop:** UVs planas, la textura incrustada, el modelo dibujado con textura.

**Verificar:**
- `cargo run -p scop -- resources/42.obj`: la imagen en el frente y la parte de atrás del
  logo, del derecho, con los colores correctos; rayas en los lados delgados (esperado).
- Tetera: la imagen por encima (estirada en los lados — esperado).
- Validación callada, también al salir. `cargo test`, `cargo clippy --all-targets`
  limpios.

---

## Preguntas

Q-14.1 (recuerdo) ¿Qué son las coordenadas de textura (UVs), qué rango usan, y de dónde
salen las de scop, si los archivos `.obj` no traen ninguna?

Q-14.2 (predicción) scop proyecta la imagen a lo largo del eje más delgado del logo (x).
¿Qué aspecto tienen las caras laterales delgadas del logo, y por qué?

Q-14.3 (predicción) Tu cargador ignora el bit de origen y el archivo está guardado con la
fila de abajo primero (lo que hace TGA por defecto). ¿Qué ves?

Q-14.4 (aplicación) La imagen pasa por un staging buffer a memoria `DEVICE_LOCAL`, con dos
transiciones de layout. ¿Por qué staging (usa tu regla de "¿cada cuánto la escribe la
CPU?"), y para qué sirve cada transición?

Q-14.5 (predicción) Creas la imagen de textura como `R8G8B8A8_UNORM` en vez de
`R8G8B8A8_SRGB`. La swapchain sigue siendo sRGB. ¿Qué aspecto tienen los colores, y por
qué?

Respuestas: `ANSWERS.md` § 14
