---
section: 9
slug: maths-for-3d
tier: T1
status: done
prereqs: [2]
terms: [vector, point vs direction, dot product, cross product, normalise, matrix, transform, homogeneous coordinates, translation matrix, rotation matrix, scale matrix, composition, model matrix, view matrix, projection matrix, perspective divide, field of view, aspect ratio, near and far planes, column-major, radians, operator overloading, epsilon comparison]
---

# Sección 9 — Las matemáticas 3D justas

**Profundidad: T1 (elección tuya)** — entender qué hace cada pieza y por qué importa el
orden, usarlo con soltura y probarlo. Sin derivaciones ni demostraciones: eso es para
ft_newton. El subject prohíbe las librerías de matemáticas, así que tendrás una pequeña
propia — y cada bug en ella aparece como un modelo en el sitio equivocado, así que los
tests importan más que la elegancia.
**Serás capaz de:** describir posiciones y direcciones con vectores, mover, girar y
dimensionar un modelo con matrices, combinarlas en el orden correcto y hacer que lo lejano
se vea más pequeño.

---

## La idea

Las esquinas de tu quad son números escritos directamente en NDC (la caja −1..+1 de § 2).
Las del logo de 42 no lo serán: las esquinas de su `.obj` están en las unidades que usara
el artista, centradas donde estuvieran, y el subject quiere que **gire sobre su propio
centro**, **visto en perspectiva**. Cada esquina hay que *moverla*, *girarla* y *aplastarla
sobre la pantalla* — cada fotograma, para cada esquina. Para eso sirven las matrices.

Empieza por las flechas. Un **vector** son tres números (x, y, z). Puede significar un
**lugar** ("la esquina está 2 a la derecha, 1 arriba, 0 hacia delante") o una **dirección**
— una flecha que dice *hacia dónde*, pero que no está en ningún sitio concreto ("el tablero
mira hacia arriba": (0, 1, 0)). Los mismos tres números, distinto significado — y esa
diferencia va a importar.

Imagina una mesa. Sus cuatro esquinas son **lugares**. "Hacia dónde mira el tablero" es una
**dirección**. Desliza la mesa por la habitación: el lugar de cada esquina cambia, pero el
tablero sigue mirando hacia arriba — la dirección *no* debe cambiar. Tumba la mesa de lado:
ahora cambian los dos — las esquinas se mueven *y* el tablero mira de lado. Así que:
**mover afecta solo a los lugares; girar afecta a los dos.**

Ahora, recetas. Imagina una tarjeta de receta que dice "coge cualquier punto y muévelo 3 a
la derecha". Otra dice "gira cualquier punto 30° alrededor del eje vertical". Una
**matriz** es una tarjeta así, escrita como una cuadrícula de 4×4 números; *aplicarla* a un
punto (multiplicar) da el punto nuevo. La magia: puedes **combinar tarjetas** en una sola
que hace las dos cosas — así que toda una cadena de "céntralo, gíralo, aléjalo de la
cámara, aplástalo sobre la pantalla" se convierte en una sola matriz, aplicada a cada
esquina en el vertex shader.

Pero combinar tiene un orden, y el orden cambia el resultado. "Gira sobre ti mismo, luego
da 3 pasos" te deja en un sitio distinto que "da 3 pasos, luego gira". Para un modelo:
*girar, luego alejar* → gira en su sitio allí lejos. *Alejar, luego girar* → da vueltas
alrededor del origen como una puerta. Es el bug 3D más común que existe.

Por último: perspectiva. Lo lejano se ve más pequeño. El truco que hace que la GPU lo haga
es casi vergonzosamente simple: **divide x e y entre la distancia**. El doble de lejos, la
mitad de grande. El trabajo de la matriz de proyección es preparar las cosas para que la
GPU haga esa división por ti.

**Dónde deja de valer la analogía:** a las tarjetas de receta en una cocina no les importa
mucho el orden. A las matrices siempre — `A × B` y `B × A` son, en general, matrices
distintas.

---

## Los nombres

**vector** — `Vec3 { x, y, z }`: tres números. Longitud = cuánto de lejos; dirección =
hacia dónde.

**point vs direction** — los mismos tres números significan un *lugar* (una esquina) o una
*dirección* (una flecha: "hacia dónde", no "dónde" — como hacia dónde mira una cara, que se
usa más adelante para la iluminación). Mover un modelo mueve sus lugares pero no debe
cambiar sus direcciones; girarlo cambia los dos. Nadie lo comprueba a mano: el 4º número de
abajo hace que las matemáticas lo hagan solas.

**dot product** — `a · b = ax·bx + ay·by + az·bz`, un número. Para dos vectores de
longitud 1 es el coseno del ángulo entre ellos: 1 = misma dirección, 0 = en ángulo recto,
−1 = opuestos. Se usa más adelante para la iluminación; útil en los tests ya.

**cross product** — `a × b`: un vector nuevo en ángulo recto con los dos. Da el "hacia
dónde mira" de una cara a partir de dos de sus aristas — lo necesitan § 12/§ 17.

**normalise** — escalar un vector a longitud 1, conservando su dirección: dividir entre su
longitud. El método se llama también `normalise` (ortografía británica, como la prosa).

**matrix** — una cuadrícula de 4×4 números: `Mat4`. Multiplicar una matriz por un vector
aplica la receta; multiplicar dos matrices crea una receta combinada.

**transform** — cualquier cambio aplicado a todos los puntos: mover, girar, redimensionar,
proyectar.

**homogeneous coordinates** — escribir un vector 3D con un **4º número `w`**: los puntos
llevan `w = 1`, las direcciones `w = 0`. Una cuadrícula de 3×3 puede girar y redimensionar
pero no puede *mover*; la 4ª columna guarda el movimiento, y se **multiplica por `w`**: para
un punto, ×1 = movido; para una dirección, ×0 = el movimiento desaparece. Ejemplo, mover
por (5, 0, 0): nueva x = x + 5·w, así que la esquina (1, 2, 0, **1**) → (6, 2, 0), pero la
flecha (0, 1, 0, **0**) → (0, 1, 0). "Homogéneas" es solo el nombre de este truco de cuatro
números.

**translation matrix** — mueve puntos por (tx, ty, tz). Los números van en la 4ª columna.

**rotation matrix** — gira puntos alrededor de un eje que pasa por el origen, un ángulo
dado. Una para cada eje X, Y, Z.

**scale matrix** — estira o encoge a lo largo de cada eje: (sx, sy, sz) en la diagonal.

**composition** — combinar matrices multiplicándolas. Con la convención de scop (abajo),
**`A × B` aplicada a un punto hace primero B y luego A** — se lee de derecha a izquierda.

**model matrix** — coloca *este modelo* en el mundo: centrarlo, girarlo, moverlo.

**view matrix** — coloca el mundo delante de la cámara. La cámara de scop está en el origen
mirando hacia −z, así que la vista es solo "empujar el mundo lejos de la cámara".

**projection matrix** — prepara la escena 3D para aplastarla en la pantalla 2D, con
perspectiva. Juntas: **`P × V × M`**, aplicadas a cada esquina (de derecha a izquierda:
modelo, luego vista, luego proyección).

**perspective divide** — tras el vertex shader, la GPU divide x, y y z entre `w`
automáticamente. La matriz de proyección pone `w` = la distancia del punto a la cámara, así
que esta división es el paso de "el doble de lejos, la mitad de grande".

**field of view** — cuánto de ancho ve la cámara, como un ángulo (vertical, en scop). Más
ancho = cabe más en pantalla, todo se ve más pequeño y más deformado en los bordes.

**aspect ratio** — ancho ÷ alto de la imagen. Dárselo a la proyección es lo que evita que
tu quad se vea estirado en una ventana ancha (la nota de § 6).

**near and far planes** — las distancias más cercana y más lejana que se dibujan. Todo lo
que quede fuera se recorta. La profundidad (§ 11) se mide entre ellos.

**column-major** — cómo se dispone una matriz de 4×4 en memoria: los 16 números guardados
*columna a columna*. GLSL lee los `mat4` así, así que scop los guarda así: `Mat4 { cols:
[[f32; 4]; 4] }`, `cols[c][r]`.

**radians** — ángulos como distancia recorrida en un círculo de radio 1: una vuelta
completa es 2π ≈ 6.28, un ángulo recto π/2. `sin`/`cos` de Rust reciben radianes.
`grados × π / 180`.

**operator overloading** — implementar los traits de `std::ops` de Rust (`Add`, `Sub`,
`Mul`) para tus tipos, para poder escribir `a + b` y `m * v` en vez de `a.add(b)`.

**epsilon comparison** — comprobar que dos floats están *cerca* (diferencia menor que un
`EPSILON` diminuto, p. ej. 1e-5) en vez de iguales. `0.1 + 0.2 != 0.3` en coma flotante;
los tests deben permitir el redondeo.

---

## Cómo funciona realmente

### 1. La convención, fijada una vez

- **Dextrógira (mano derecha)**: x a la derecha, y arriba, z hacia ti (la cámara mira
  hacia −z). Es la convención 3D habitual y la que asumen los archivos `.obj`.
- **Vectores columna**: los puntos son columnas, las matrices multiplican por la
  izquierda: `M × p`. La composición se lee **de derecha a izquierda**.
- **Almacenamiento column-major**, como GLSL.
- **La pantalla de Vulkan es distinta**: y hacia abajo, profundidad 0..1. La matriz de
  proyección se encarga de las dos cosas, así que todo lo de *antes* de ella se queda en el
  amable mundo dextrógiro.

Escribe estas cuatro líneas como comentario al principio de `maths.rs`; todo informe de bug
futuro empieza ahí.

### 2. `Vec3`

`new`, `+`, `−`, `* f32`, `dot`, `cross`, `length`, `normalise`. Un simple
`#[derive(Clone, Copy, Debug, PartialEq)]` — números, que no poseen nada.

### 3. `Mat4` — las tarjetas

Escritas en la disposición matemática habitual (filas en el papel). En memoria, scop guarda
las *columnas*.

```
identidad           traslación(t)        escala(s)
1 0 0 0             1 0 0 tx             sx 0  0  0
0 1 0 0             0 1 0 ty             0  sy 0  0
0 0 1 0             0 0 1 tz             0  0  sz 0
0 0 0 1             0 0 0 1              0  0  0  1

rotation_y(a)       (c = cos a, s = sin a; parecido para x y z)
 c  0  s  0
 0  1  0  0
-s  0  c  0
 0  0  0  1
```

No necesitas saber *por qué* son esas entradas; necesitas saber **qué tarjeta hace qué**, y
probarlo: girar (1, 0, 0) un cuarto de vuelta alrededor de y da (0, 0, −1).

Multiplicar `A × B`: la entrada (fila r, columna c) del resultado = producto escalar de la
fila r de A con la columna c de B. Dieciséis productos escalares; un bucle.

### 4. Perspectiva, al estilo Vulkan

```
f = 1 / tan(fov_y / 2)

f/aspect   0      0                    0
0         -f      0                    0       ← menos: la y de Vulkan apunta hacia abajo
0          0      far/(near−far)       near·far/(near−far)
0          0     -1                    0       ← w = −z = distancia delante de la cámara
```

Lo que hay que recordar, no derivar: la última fila copia la distancia en `w` (así la
división de la GPU hace más pequeño lo lejano); la tercera fila comprime las distancias
entre near y far en el rango de profundidad 0..1 de Vulkan; `−f` invierte la y para Vulkan;
`/aspect` deshace el estiramiento de la ventana. Un test lo fija: un punto a distancia
`near` recibe profundidad 0, a `far` profundidad 1.

### 5. Poner un modelo delante de la cámara

Para el "girar sobre su propio centro" del subject:

```
model = translation(position) × rotation_y(angle) × translation(−centre)
           3º: ponerlo en su sitio    2º: girar      1º: centrarlo en el origen
view  = translation(0, 0, −distance)   // empujar el mundo delante de la cámara
mvp   = projection × view × model
```

Se lee de derecha a izquierda: centrar → girar → colocar → alejar → proyectar. § 10 envía
`mvp` al vertex shader; § 13 añade las teclas.

### 6. Los tests son lo importante

Los floats nunca se comparan iguales de forma fiable, así que un helper
`assert_close(a, b)` con un epsilon. Prueba cada tarjeta con un punto conocido,
`identity × m == m`, que el orden importa (`T×R ≠ R×T`), las profundidades near/far de la
perspectiva y que `normalise` da longitud 1.

---

## Trampas conocidas

- **El modelo da vueltas alrededor en vez de girar en su sitio** — el orden de composición
  está al revés (`R × T` donde querías `T × R`). Lee de derecha a izquierda.
- **Modelo boca abajo, o se mueve al revés en vertical** — la inversión de y hecha dos
  veces, o ninguna. Está solo en la proyección.
- **Todo reflejado o del revés** — un error de signo en una rotación, o una matriz
  levógira mezclada con dextrógiras. Prueba cada rotación con un punto.
- **Basura o nada en pantalla tras enviar una matriz** — datos row-major leídos como
  column-major (la matriz está traspuesta: la traslación acaba en la fila de abajo, donde
  le hace cosas raras a `w`).
- **Rotación 57× demasiado rápida** — grados donde se esperan radianes.
- **Los tests fallan con `0.30000001 != 0.3`** — usa una comparación con epsilon.
- **`normalise` de un vector de longitud cero** — divide entre cero → `NaN` por todas
  partes, modelo invisible. Protégelo.
- **Modelo estirado** — falta el aspect ratio o está invertido (alto ÷ ancho).

---

## Checkpoint

**Cumple:** nada visible todavía — las matemáticas en que se apoyan "perspectiva" y "girar
sobre su centro", con tests unitarios. (Método: primero el gate, luego Claude lo escribe y
te lo explica.)

1. `engine/src/maths.rs`: el comentario de la convención; `Vec3` con `+ − * dot cross
   length normalise`; `Mat4` (column-major) con `identity`, `translation`, `scale`,
   `rotation_x/y/z`, `perspective`, `Mat4 × Mat4`, `Mat4 × punto`.
2. Tests unitarios para cada función, con un helper de epsilon — incluidas la de "el orden
   importa" y las comprobaciones de profundidad de la perspectiva.
3. Reexportado desde el engine (`engine::Vec3`, `engine::Mat4`).
4. **Demuestra que el orden importa:** un test de que `translation × rotation` y
   `rotation × translation` llevan el mismo punto a sitios distintos.

**Verificar:** `cargo test` — todos los tests de matemáticas pasan; `cargo clippy` limpio.
Nada cambia en pantalla hasta § 10.

---

## Preguntas

Q-9.1 (recuerdo) ¿Por qué un engine 3D usa matrices de 4×4 y un 4º número `w`? ¿Qué `w`
recibe un punto, qué `w` recibe una dirección, y qué diferencia hay al aplicar una
traslación?

Q-9.2 (predicción) El modelo está en (3, 0, 0). Construyes `model = rotation_y(angle) ×
translation(3, 0, 0)` y aumentas `angle` cada fotograma. ¿Qué hace el modelo en pantalla?
¿Qué querías probablemente, y cómo se escribe?

Q-9.3 (aplicación) Las esquinas del logo de 42 están centradas en (2, 1, 0), no en el
origen. Escribe la matriz de modelo (como producto de tarjetas) que lo hace girar sobre su
*propio* centro, en su sitio en el origen.

Q-9.4 (predicción) Dos cubos idénticos, uno a 2 unidades delante de la cámara y otro a 4.
¿Más o menos de qué tamaño se ve el lejano comparado con el cercano, y qué paso hace que
ocurra?

Q-9.5 (predicción) Tu código en Rust guarda las matrices fila a fila, pero GLSL las lee
columna a columna. Nada más está mal. ¿Qué le pasa a una traslación, y qué podrías ver?

Respuestas: `ANSWERS.md` § 9
