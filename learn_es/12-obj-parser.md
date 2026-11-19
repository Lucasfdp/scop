---
section: 12
slug: obj-parser
tier: T1
status: done
prereqs: [11]
terms: [Wavefront OBJ, vertex line, face line, 1-based index, negative index, face token, texture coordinate and normal lines, material library, triangulation, fan triangulation, convex and concave, bounding box, model centre, framing, parse error with line number]
---

# Sección 12 — Leer archivos .obj

**Profundidad: T1** — el formato es sencillo y solo necesitas una parte. Lo que importa es
leerlo *correctamente* (el clásico bug es el off-by-one) y de forma *robusta*: el subject
avisa de que los evaluadores cargarán otros archivos `.obj` durante la defensa.
**Serás capaz de:** cargar el logo de 42 y la tetera desde disco, convertirlos en
`MeshData` y verlos girar, centrados y cabiendo en la ventana.

---

## La idea

Un archivo `.obj` es solo texto, una instrucción por línea. Este es el principio de
`resources/42.obj`:

```
# Blender v2.71 (sub 0) OBJ File: '42.blend'
mtllib 42.mtl
o Cube
v 0.232406 -1.216630 1.133818
v 0.232406 -0.745504 2.843098
...
f 1 2 3
f 16 2 3 17
```

Dos tipos de línea hacen todo el trabajo:

- **`v x y z`** — "aquí hay una esquina". Las esquinas se numeran en el orden en que
  aparecen: la primera línea `v` es la esquina **1**, la segunda la **2**, y así.
- **`f a b c …`** — "aquí hay una cara, hecha con estas esquinas". `f 1 2 3` es un
  triángulo con las esquinas 1, 2 y 3. `f 16 2 3 17` es una cara con **cuatro** esquinas.

Eso es exactamente lo que ya quiere tu `MeshData` (§ 7): una lista de esquinas, y una lista
de índices donde cada 3 forman un triángulo. Todo el trabajo del parser es traducir lo uno
a lo otro. Dos pequeñas diferencias estorban:

1. El archivo cuenta desde **1**; los arrays de Rust cuentan desde **0**. Cada índice
   necesita `− 1`.
2. Las caras pueden tener 4 (o más) esquinas; la GPU solo dibuja triángulos. Una cara de 4
   esquinas se corta en 2 triángulos.

Todo lo demás del archivo (comentarios, nombres de objeto, materiales, grupos de suavizado)
scop se lo puede **saltar**. El subject solo pide forma, perspectiva, grises y una textura —
y los grises vienen del subject, no del archivo de materiales.

Con el modelo cargado quedan dos problemas prácticos. Las esquinas del logo van de z = 0 a
z = 2.84, así que su centro no está en el origen — girarlo lo haría dar vueltas como la
puerta de Q-9.2. Y la tetera mide unas 6 unidades de ancho, así que una cámara a 3
unidades estaría *dentro* de ella. Las dos cosas se arreglan con una sola medida: la caja
más pequeña que contiene todas las esquinas. Su centro es el centro del modelo; su tamaño le
dice a la cámara a qué distancia ponerse.

**Dónde deja de valer la analogía:** "cada 3 índices forman un triángulo" es *tu* formato;
`.obj` no tiene esa regla — una línea de cara puede tener cualquier número de esquinas, y
algunas cosas de `.obj` (índices negativos, tokens `1/2/3`) no se parecen en nada a números
normales. El parser tiene que manejar el formato tal como está escrito, no como te gustaría
que fuera.

---

## Los nombres

**Wavefront OBJ** — el formato de texto `.obj`, del software de animación de Wavefront
Technologies de los años 80. Sigue en todas partes porque es trivial de leer y escribir.

**vertex line** — `v x y z`: la posición de una esquina. La `v` es de **v**ertex (vértice).

**face line** — `f a b c …`: un polígono, que lista sus esquinas por número. La `f` es de
**f**ace (cara).

**1-based index** — contar empezando en 1, no en 0. Los números de esquina de `.obj` empiezan
en 1, así que la esquina `n` es `positions[n − 1]`.

**negative index** — `f -3 -2 -1` significa "las tres últimas esquinas definidas *hasta
ahora*": −1 es la línea `v` más reciente. Raro, pero legal — el archivo de un evaluador
podría usarlo. La esquina `−k` = `positions[len − k]` en el momento en que se lee la cara.

**face token** — cada elemento tras la `f`. Puede ser `5`, `5/2`, `5/2/7` o `5//7`: esquina
/ coordenada de textura / normal. scop solo necesita la parte antes de la primera `/`.

**texture coordinate and normal lines** — `vt u v` (dónde de una imagen cae esta esquina;
§ 14 crea las suyas, § 18 puede que lea estas) y `vn x y z` (hacia dónde mira la
superficie — una *dirección*, el `w = 0` de § 9). Saltadas por ahora.

**material library** — `mtllib 42.mtl` y `usemtl Material`: colores y brillo en un archivo
`.mtl` aparte. Saltado: el subject quiere grises, no los colores del archivo.

**triangulation** — cortar un polígono en triángulos.

**fan triangulation** — la forma más sencilla: dejar fija la primera esquina y emparejarla
con cada arista siguiente. Esquinas `a b c d e` → `a b c`, `a c d`, `a d e`. Como abrir un
abanico de papel desde un punto.

**convex and concave** — *convexo*: todas las esquinas apuntan hacia fuera, como un
cuadrado o un hexágono; un abanico siempre funciona. *Cóncavo*: alguna esquina apunta hacia
dentro, como una L o una flecha; un abanico puede producir triángulos que se salen *fuera*
de la forma. Tratarlos bien es el bonus de § 17.

**bounding box** — la caja más pequeña, alineada con los ejes x/y/z, que contiene todas las
esquinas: el mínimo y el máximo de cada coordenada.

**model centre** — el centro de la bounding box: `(min + max) / 2`. Es lo que el subject
llama "el centro del objeto" para la rotación.

**framing** — elegir la distancia de la cámara para que el modelo entero quepa en la
ventana.

**parse error with line number** — `42.obj:17: la cara usa la esquina 50, pero solo hay 42
definidas`. Un archivo roto de un evaluador debe dar un mensaje claro y una salida limpia,
nunca un panic ni un crash.

---

## Cómo funciona realmente

### 1. Dónde vive

El parser va en el **engine** (`engine/src/obj.rs`), porque ft_newton, ft_vox y compañía
también cargarán archivos `.obj`. No sabe nada de colores ni de scop:

```rust
pub struct ObjModel {
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,   // desde 0, listos para la GPU
    pub min: Vec3,                  // bounding box
    pub max: Vec3,
}
impl ObjModel {
    pub fn centre(&self) -> Vec3;   // (min + max) / 2
    pub fn radius(&self) -> f32;    // la mitad de la diagonal de la caja: "cuánto de grande"
}
pub fn parse_obj(text: &str) -> Result<ObjModel, ObjError>;
```

Después scop convierte un `ObjModel` en `MeshData` — elegir los colores es cosa de scop (y
el tema de § 13).

### 2. Línea a línea

```
for (line_number, line) in text.lines().enumerate():
    quitar todo desde '#'; partir por espacios
    según la primera palabra:
        "v"  → leer 3 floats (ignorar un 4º, si lo hay) → positions.push
        "f"  → leer cada token:
                 coger la parte antes de '/'
                 parsear como entero n
                 n > 0 → index = n − 1          (empieza en 1)
                 n < 0 → index = len + n        (relativo: −1 = el último hasta ahora)
                 n = 0 o fuera de rango → error con número de línea
               menos de 3 esquinas → error
               abanico: for i in 1..count−1 → triángulo (c0, c[i], c[i+1])
        cualquier otra cosa → saltar   (o, g, s, mtllib, usemtl, vt, vn, l, …)
    ninguna cara al final → error ("nada que dibujar")
```

`text.lines()` también quita los finales de línea `\r\n` de Windows, así que los archivos
guardados en Windows funcionan.

### 3. Centro y encuadre

Tras parsear: bounding box → centro → radio. En scop:

```
model  = rotation_y(angle) × translation(−centre)        // Q-9.3: centrar, luego girar
distance = radius / sin(fov_y / 2)                        // lo bastante lejos para que quepa
                                                          // la esfera entera que lo rodea
view   = translation(0, 0, −distance)
far    = distance + radius                                // para que no se recorte la parte de atrás
```

No necesitas derivar la línea del `sin`: "modelo más grande o vista más estrecha →
ponerse más atrás" es toda la idea.

### 4. Los errores llegan limpios al usuario

`ObjError` lleva el número de línea y un motivo, e implementa `Display`. `main` imprime
`scop: resources/broken.obj:17: …` y devuelve un código de salida de fallo — el mismo camino
que hoy un archivo que no existe. Ningún `unwrap` sobre nada que venga del archivo.

### Decisión: qué soporta el parser

| Soportado ya | Saltado (y por qué) |
|---|---|
| `v`, `f` con cualquier número de esquinas | `vt` / `vn`: no hacen falta hasta § 14/18 |
| tokens `5/2/7`, `5//7` (se usa la primera parte) | `mtllib` / `usemtl`: el subject quiere grises |
| índices negativos | `l` (líneas), `p` (puntos): nada que dibujar como triángulos |
| comentarios, líneas vacías, `\r\n` | polígonos cóncavos dibujados correctamente: bonus de § 17 |

---

## Trampas conocidas

- **El modelo es una maraña de triángulos puntiagudos** — falta el `− 1` (o se aplica dos
  veces): cada cara une las esquinas equivocadas. (Q-12.3)
- **Panic `index out of bounds`** — el mismo bug, la última esquina; o el archivo de un
  evaluador con un índice malo. El parser debe comprobarlo y devolver un error.
- **Agujeros en el modelo, o solo media cara de cada quad dibujada** — caras de 4 esquinas
  tomadas como un triángulo (solo las 3 primeras esquinas). Triangula en abanico.
- **Triángulos raros saliendo de una cara** — triangulación en abanico sobre una cara
  cóncava. Vale para los archivos obligatorios; es el bonus de § 17.
- **El modelo da vueltas en vez de girar** — no está centrado (Q-9.2/9.3).
- **No se ve nada, o solo un trozo del modelo** — cámara dentro del modelo o plano lejano
  demasiado cerca: usa la distancia de encuadre y `far = distance + radius`.
- **El modelo está tumbado** — el "arriba" del archivo no es +y (el arriba propio de
  Blender es +z). No es un bug del parser; una tarjeta de rotación fija en scop si importa
  (§ 13).
- **Panic con un archivo raro en vez de un mensaje** — un `unwrap()` sobre un resultado de
  parseo. Todo fallo debe convertirse en un `ObjError` con número de línea.

---

## Checkpoint

**Cumple (subject):** "parsear el .obj y renderizarlo correctamente"; "el origen en el
centro del objeto". Método: primero el gate, luego Claude lo escribe y te lo explica.

1. **Engine:** `engine/src/obj.rs` — `parse_obj`, `ObjModel` (posiciones, triángulos desde
   0, bounding box, `centre`, `radius`), `ObjError` con números de línea. Tests unitarios:
   triángulo, quad → 2 triángulos, tokens `1/2/3`, índices negativos, comentarios/líneas
   desconocidas saltadas, índice 0 / fuera de rango / muy pocas esquinas / sin caras →
   errores.
2. **scop:** leer el archivo de la línea de comandos, parsearlo, imprimir `n esquinas, m
   triángulos`, construir `MeshData` (color por esquina según su posición, como el cubo —
   los grises de verdad son § 13), centrarlo, encuadrarlo, girarlo. Un error de parseo →
   mensaje + código de salida de fallo.
3. Se quita el código del cubo.

**Verificar:**
- `cargo run -p scop -- resources/42.obj`: el logo, girando sobre su propio centro, visible
  entero.
- `… resources/teapot.obj` y `teapot2.obj`: la tetera, igual.
- Un archivo roto (`echo "f 1 2 3" > /tmp/bad.obj`): un mensaje claro, sin panic.
- `cargo test`, `cargo clippy --all-targets` limpios; validación callada.

---

## Preguntas

Q-12.1 (recuerdo) ¿Qué dos tipos de línea usa de verdad el parser de scop, y qué hace con
todas las demás líneas? Da dos ejemplos de líneas que se salta.

Q-12.2 (aplicación) La línea es `f 16 2 3 17`. ¿Qué triángulos (como índices desde 0)
produce el parser?

Q-12.3 (predicción) Olvidas el `− 1`. ¿Qué aspecto tendría el logo de 42, y qué pasa cuando
el parser llega a una cara que usa la esquina 42?

Q-12.4 (predicción) Una cara tiene forma de L (6 esquinas, una apuntando hacia dentro) y se
triangula en abanico desde su primera esquina. ¿Qué puede salir mal en pantalla?

Q-12.5 (aplicación) Las esquinas del logo van de z = 0 a z = 2.84. Escribe la matriz de
modelo (como tarjetas) que lo hace girar sobre su propio centro en el origen, y di de dónde
sale el centro.

Respuestas: `ANSWERS.md` § 12
