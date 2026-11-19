---
section: 13
slug: faces-and-controls
tier: T1
status: done
prereqs: [12]
terms: [flat colour per face, shared corners, duplicated corners, face of a triangle, spread-out greys, held vs pressed, per-second speed, model state, model matrix recipe, world-space movement, Euler angles, gimbal lock]
---

# Sección 13 — Caras grises y mover el modelo

**Profundidad: T1** — ningún concepto nuevo de GPU: esta sección va de *disposición de
datos* (por qué el color por cara necesita más esquinas) y de *estado de la app* (las teclas
cambian números; los números construyen la matriz de § 9). Lo justo para terminar los dos
requisitos de "aspecto y manejo" que le quedan al subject y explicarlos en la defensa.
**Serás capaz de:** mostrar cada cara del logo en su propio gris sutil, y moverlo y girarlo
en los tres ejes con el teclado, siempre sobre su propio centro.

---

## La idea

Faltan todavía dos requisitos del subject:

1. *"Usando varios colores, debes hacer que las distintas caras del objeto se distingan
   visualmente"* — y *"las caras deben tener sutiles tonos de gris"*.
2. *"Debes poder rotarlo y trasladarlo alrededor de sus tres ejes principales de simetría
   (con el origen en el centro del objeto), con cualquier método de entrada del usuario."*

**Grises.** Ahora mismo cada *esquina* tiene un color, y la GPU mezcla los colores a lo
largo de cada triángulo (§ 6: el rasterizador interpola). Por eso el logo parece un arcoíris
suave. Queremos lo contrario: un gris plano por cara, con un borde visible donde se tocan dos
caras.

La pega es que una esquina se **comparte**: la esquina 2 del logo pertenece a varias caras.
Si la esquina 2 solo puede tener un color, toda cara que la toque recibe *ese* color en ese
punto, y los grises se mezclan entre sí. Imagina un mosaico donde cada tesela debe ser de un
color, pero las teselas vecinas comparten los clavos de sus esquinas — y cada clavo está
pintado de un color. No puedes distinguir las teselas.

El arreglo: dejar de compartir. Darle a cada triángulo **sus propias tres esquinas**, copias
de las originales, todas pintadas del gris del triángulo. Mismas posiciones, más esquinas.
La tetera pasa de 3644 esquinas a 18 960 — unos 450 KB de memoria de GPU. Nada.

¿Qué gris para qué cara? Si contaras sin más 0.30, 0.31, 0.32…, las caras vecinas (que
suelen ser vecinas también en el archivo) tendrían casi el mismo gris. En su lugar, salta
por el rango a pasos que nunca se alinean — para que caras consecutivas caigan siempre
lejos — y mantén todos los grises en una franja media: sutiles, nunca negro ni blanco.

**Controles.** La colocación del modelo son solo unos pocos números: dónde está (x, y, z) y
cuánto está girado en cada eje. Las teclas cambian esos números un poco cada fotograma;
`draw` construye a partir de ellos la matriz de modelo de § 9. La receta ya la has escrito:

```
model = muévelo × gíralo × céntralo
```

**Dónde deja de valer la analogía:** "copias de las esquinas" suena a derroche, pero es la
forma normal en que las GPUs dibujan modelos de color plano o aristas marcadas — lo mismo
pasa más adelante siempre que una esquina necesite valores distintos para caras distintas
(normales para la iluminación, coordenadas de textura en una costura).

---

## Los nombres

**flat colour per face** — todos los píxeles de una cara reciben el mismo color; sin mezcla
a lo largo de ella. Aquí se consigue dando a las tres esquinas de un triángulo el mismo
color, para que la mezcla del rasterizador no tenga nada que mezclar.

**shared corners** — la disposición de § 7/§ 12: cada posición se guarda una vez, y la usan
todos los triángulos que la tocan a través de la lista de índices. Compacta; un valor por
esquina.

**duplicated corners** — (disposición "sin compartir", "flat-shaded"): cada triángulo recibe
sus propias tres esquinas. Tres vértices por triángulo; cada esquina puede llevar un valor
que pertenece a *una sola* cara.

**face of a triangle** — de qué línea de cara del `.obj` salió un triángulo. Una cara de 4
esquinas se convierte en 2 triángulos (§ 12); los dos deben recibir el *mismo* gris, así que
el parser apunta el número de cara de cada triángulo.

**spread-out greys** — elegir el gris de cada cara avanzando por un rango a pasos de una
fracción irracional (aquí 0.618…, la fracción del *número áureo*), para que caras
consecutivas reciban grises muy distintos y ningún patrón se repita. El rango se limita a
una franja media para que sean "tonos sutiles".

**held vs pressed** — `input.held(key)`: true **cada fotograma** que la tecla está abajo
(para movimiento continuo). `input.pressed(key)`: true solo **en el fotograma en que baja**
(para interruptores: una pulsación = un cambio).

**per-second speed** — movimiento escrito como "unidades por segundo" y multiplicado por
`dt` (§ 10): la misma velocidad a cualquier tasa de fotogramas.

**model state** — los números que describen dónde está el modelo y cómo está girado:
posición (x, y, z) y tres ángulos. Vive en `Scop`, cambia en `update`, se lee en `draw`.

**model matrix recipe** — `translation(position) × rotation × translation(−centre)`. De
derecha a izquierda: centrarlo en el origen, girarlo ahí, y luego llevarlo a `position`.

**world-space movement** — moverse según los ejes de la pantalla (derecha siempre es
derecha), esté como esté girado el modelo. Sale de poner la tarjeta de traslación *la más a
la izquierda*: ocurre después del giro. (Ponla a la derecha del giro y "derecha" significa
"la derecha del propio modelo" — Q-13.4.)

**Euler angles** — describir un giro como tres ángulos, uno por eje, aplicados en un orden
fijo (aquí: z, luego x, luego y). Llevan el nombre del matemático del siglo XVIII Leonhard
Euler. Sencillos y suficientes para un visor.

**gimbal lock** — la rareza conocida de los ángulos de Euler: en algunas combinaciones (p.
ej. 90° alrededor de x) dos de las teclas acaban girando el modelo alrededor del mismo eje
de pantalla. Inofensivo aquí; los engines de verdad usan *cuaterniones* (T0: una forma de 4
números de guardar un giro) para evitarlo — terreno de ft_newton.

---

## Cómo funciona realmente

### 1. El parser apunta las caras

`ObjModel` gana una lista, rellenada por el mismo bucle del abanico:

```rust
pub triangle_faces: Vec<u32>,   // triangle_faces[t] = de qué línea de cara salió el triángulo t
```

Un quad mete el mismo número de cara dos veces. Nada más cambia en el parser.

### 2. scop construye esquinas duplicadas y grises

```
para cada triángulo t con esquinas [a, b, c]:
    grey = face_grey(triangle_faces[t])
    meter 3 vértices: positions[a], positions[b], positions[c], todos con color (grey, grey, grey)
indices = 0, 1, 2, 3, 4, 5, …       // cada vértice se usa exactamente una vez
```

```
face_grey(i) = 0.25 + 0.5 × fract(i × 0.618034)     // siempre en 0.25..0.75: sutil
```

`fract` se queda con la parte tras el punto decimal. `i × 0.618…` salta algo más de la
mitad cada vez y nunca se repite, así que las caras 0, 1, 2, 3… reciben grises como 0.25,
0.56, 0.37, 0.68… — las vecinas siempre se distinguen.

Ningún cambio en el shader: las tres esquinas de un triángulo tienen el mismo color, así
que la interpolación entre ellas produce ese mismo color en todas partes.

### 3. Controles

```
mantener ← → ↑ ↓     mover izquierda/derecha/arriba/abajo   (x, y)
mantener W S         acercar / alejar                        (z)   (letras: + y − están en otro sitio en un teclado español)
mantener I K         girar alrededor de x   (inclinar)
mantener J L         girar alrededor de y   (girar)
mantener U O         girar alrededor de z   (alabear)
pulsar Space         pausar / reanudar el giro automático
pulsar R             reiniciar posición y ángulos
pulsar Esc           salir
```

Las velocidades son por segundo (× `dt`); el movimiento va también × el `radius` del
modelo, así que una tetera grande y un logo pequeño se mueven ambos "más o menos una
quinta parte de su tamaño por segundo".

### 4. La matriz de modelo, cada fotograma

```rust
model = translation(position)
      × rotation_y(spin + angle_y)
      × rotation_x(angle_x)
      × rotation_z(angle_z)
      × translation(−centre)
```

De derecha a izquierda: centrar → alabear (z) → inclinar (x) → girar (y, más el giro
automático) → mover. El "rotar alrededor de sus ejes principales, origen en el centro" del
subject es la última tarjeta trabajando con las tres tarjetas de giro; "trasladar" es la
primera.

### Decisión: por cara del `.obj`, no por triángulo

| Opción | A favor | En contra |
|---|---|---|
| **Gris por cara del `.obj`** ✅ | Un quad se ve como una cara, como lo dibujó el modelador | Una lista más en el parser |
| Gris por triángulo | Ningún cambio en el parser; el subject lo permite | Cada quad muestra una costura diagonal entre dos grises |
| `gl_PrimitiveID` en el fragment shader (sin esquinas duplicadas) | Sin vértices extra | Necesita la feature de geometry shader, que MoltenVK no soporta |

---

## Trampas conocidas

- **Las caras se siguen mezclando entre sí** — colores puestos por esquina en la mesh
  *compartida* (Q-13.1). Duplica las esquinas.
- **El modelo es un lío de triángulos equivocados tras duplicar** — los índices siguen
  apuntando a las posiciones originales; tras duplicar, los índices deben ser
  `0, 1, 2, 3, …`.
- **Cada quad muestra una línea diagonal** — gris elegido por triángulo, no por cara.
- **Las teclas lo mueven a saltos, o nada** — `pressed` usado para el movimiento (un paso
  por pulsación) o `held` usado para un interruptor (cambia cada fotograma mientras se
  mantiene: parece aleatorio).
- **Más rápido en una máquina más rápida** — una velocidad no multiplicada por `dt`.
- **Las flechas lo mueven en direcciones raras tras girarlo** — la tarjeta de traslación
  está a la derecha de la rotación (movimiento en el espacio del modelo, Q-13.4).
- **Da vueltas en vez de girar en su sitio** — falta la tarjeta `translation(−centre)` o no
  está la más a la derecha.
- **Dos teclas parecen girarlo igual** — gimbal lock en ese ángulo. Esperado; pulsa R.

---

## Checkpoint

**Cumple (subject):** "caras visualmente distinguibles", "sutiles tonos de gris", "rotar y
trasladar alrededor de sus tres ejes principales … origen en el centro".
Método: primero el gate, luego Claude lo escribe y te lo explica.

1. **Engine:** `ObjModel::triangle_faces`, rellenado por el bucle del abanico; un test de
   que un quad da dos triángulos con el mismo número de cara.
2. **scop:** esquinas duplicadas con un gris por cara (`face_grey`), índices secuenciales;
   un test de que `face_grey` se queda en 0.25..0.75 y que los vecinos difieren.
3. **scop:** estado del modelo (posición, tres ángulos, giro, giro activado/desactivado), la
   tabla de teclas de arriba en `update`, la matriz de modelo de 5 tarjetas en `draw`.
   Imprimir los controles una vez al arrancar.

**Verificar:**
- `cargo run -p scop -- resources/42.obj`: cada cara de un gris plano, aristas visibles.
- Cada tecla hace lo que dice la tabla; las flechas siempre mueven según los ejes de la
  pantalla.
- Space pausa / reanuda; R reinicia.
- Tetera: lo mismo, a una velocidad cómoda.
- `cargo test`, `cargo clippy --all-targets` limpios; validación callada.

---

## Preguntas

Q-13.1 (predicción) Le das un gris a cada cara, pero mantienes la mesh de § 12 donde cada
esquina se guarda una vez y se comparte. ¿Qué aspecto tiene el logo, y por qué?

Q-13.2 (recuerdo) Tras duplicar las esquinas, ¿cuántos vértices envía la tetera (6320
triángulos) a la GPU, y qué aspecto tienen sus índices?

Q-13.3 (aplicación) Las flechas usan `held`, pero Space usa `pressed`. ¿Por qué cada una?
¿Qué saldría mal con Space si usara `held`?

Q-13.4 (predicción) Escribes la matriz de modelo como `rotation_y(a) × translation(position)
× translation(−centre)`. Giras el modelo 90° alrededor de y y mantienes →. ¿Hacia dónde se
mueve en pantalla, y por qué?

Q-13.5 (aplicación) La velocidad de movimiento es `0.2 × radius × dt` por fotograma. ¿Por
qué cada uno de los tres factores?

Respuestas: `ANSWERS.md` § 13
