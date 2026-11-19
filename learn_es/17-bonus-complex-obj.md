---
section: 17
slug: bonus-complex-obj
tier: T1
status: done
prereqs: [16]
terms: [reflex corner, ear, ear clipping, polygon normal, Newell's method, flattening a polygon, point-in-triangle test, self-intersecting polygon, open edge, area check]
---

# Sección 17 — Caras cóncavas y no planas (bonus)

**Profundidad: T1** — un algoritmo (ear clipping), una medida (aristas abiertas), y el
producto vectorial de § 9 aplicado a una cara entera. Bonus (10 pts): *"Maneja archivos
`.obj` más complejos: caras cóncavas y no coplanares; renderiza la tetera original sin
artefactos en los bordes."*
**Serás capaz de:** dividir cualquier cara en los triángulos correctos, tenga la forma que
tenga, y demostrarle a un evaluador si un hueco en un modelo es culpa de scop o del
archivo.

> _Este archivo se reescribió el 2026-11-19 a partir del código
> (`engine/src/triangulate.rs`, `ObjModel::open_edges`), `DECISIONS.md` (2026-11-18),
> `PROGRESS.md` y las respuestas y entradas de glosario de esta sección, después de que el
> original se perdiera en un commit._

---

## La idea

Desde § 12, una cara con más de tres esquinas se corta en **abanico**: fijar la primera
esquina y emparejarla con cada arista siguiente. Eso siempre está bien para una cara
*convexa* (todas las esquinas apuntan hacia fuera, como un cuadrado o un hexágono). Falla en
cuanto una cara tiene una esquina que apunta **hacia dentro** — una L, una flecha, una
estrella. Entonces algunos triángulos del abanico cruzan la muesca y cubren área que no
forma parte de la cara.

El arreglo es cortar la forma como la recortarías de un papel con tijeras: busca una
esquina que puedas cortar *limpiamente* — su triángulo queda entero dentro de la forma —,
quédate con ese triángulo y sigue con la forma más pequeña que queda. Una esquina así se
llama **oreja** (ear). Corta orejas una a una hasta que solo queden tres esquinas: ese es el
último trozo. Cada trozo que cortaste es parte de la cara, y nada de fuera lo es.

Recórrelo con la L (esquinas A B C D E F, dando la vuelta): no es seguro empezar en A — el
triángulo A-C-D cruzaría la muesca —, pero la esquina en B es una oreja: corta A-B-C. Ahora
la forma es A C D E F, y C, que antes apuntaba hacia dentro, se ha convertido en oreja.
Corta, corta, y las tres últimas esquinas son el último triángulo. Cuatro triángulos en
total — siempre `esquinas − 2`.

Primero, dos problemas prácticos:

1. **"Dentro" y "apunta hacia fuera" son ideas 2D**, pero las caras de un `.obj` viven en
   3D, en cualquier ángulo, y algunas están un poco **dobladas** (no todas las esquinas en
   un mismo plano). Así que primero averiguas hacia dónde mira la cara — su **normal**, la
   flecha que sale de ella, como el tablero de mesa de § 9 — y luego la miras de frente
   desde ese lado, quitando la tercera coordenada. Eso da un contorno plano en 2D sobre el
   que hacer las pruebas.
2. **Caras rotas.** Un archivo puede contener una "cara" cuyas aristas se cruzan (una
   pajarita). No tiene un interior de verdad, así que puede que no haya ninguna oreja. El
   bucle tiene que terminar igualmente: cuando no se encuentra oreja, se hace un abanico
   con lo que quede y se sigue. La cara se dibuja y el programa nunca se cuelga.

Y luego la tetera. El subject dice "sin artefactos en los bordes", pero la tetera original
tiene una ranura visible alrededor de la tapa. ¿Es culpa de scop? La forma de responder es
**medir**: una arista que usa un solo triángulo es una **arista abierta** — un borde donde la
superficie se acaba y se puede ver a través. Un modelo cerrado no tiene ninguna.

**Dónde deja de valer la analogía de las tijeras:** el papel siempre es plano. Una cara
doblada no tiene un único contorno "verdadero"; el aplanado es una buena aproximación, y los
triángulos siguen usando las esquinas 3D reales — solo la *elección* de los cortes se hace
en 2D.

---

## Los nombres

**reflex corner** — una esquina que apunta hacia dentro (su ángulo interior pasa de 180°).
Solo las tienen las caras cóncavas; son lo que rompe el abanico.

**ear** — tres esquinas consecutivas cuyo triángulo queda entero dentro del polígono: la
esquina del medio gira hacia fuera, y ninguna otra esquina está dentro del triángulo. Se
puede cortar sin peligro. La oreja es el triángulo que te **quedas**, no un trozo que tiras.

**ear clipping** — triangular cortando orejas de una en una hasta que queda un triángulo.
Siempre `n − 2` triángulos para `n` esquinas.

**polygon normal** — la dirección hacia la que mira una cara: una *dirección* (`w = 0`,
§ 9), no un lugar.

**Newell's method** — una normal de polígono calculada a partir de **todas** las aristas de
la cara (tres sumas acumuladas), que lleva el nombre de Martin Newell (el de la tetera de
Utah). Un solo producto vectorial de dos aristas solo ve tres esquinas: si la del medio es
una esquina reflex, apunta al revés. Las sumas de Newell no se dejan engañar por una esquina
hacia dentro, y siguen dando una media razonable para una cara un poco doblada.

**flattening a polygon** — quitar la coordenada (x, y o z) hacia la que más apunta la
normal, para que la cara se convierta en un contorno 2D visto de frente. Si la cara se ve
por detrás, el contorno se refleja para que "gira a la izquierda" siga significando "apunta
hacia fuera".

**point-in-triangle test** — un punto 2D está dentro de un triángulo si está del mismo lado
(la izquierda, para un triángulo antihorario) de las tres aristas. "Qué lado" es un producto
vectorial 2D — el mismo `turn(a, b, c)` que se usa para ver si una esquina apunta hacia
fuera.

**self-intersecting polygon** — una cara cuyas aristas se cruzan, como una pajarita. No es
una cara válida; el ear clipping puede quedarse sin orejas, así que scop hace un abanico con
el resto.

**open edge** — una arista que usa **un solo** triángulo: la superficie tiene un borde ahí.
En un modelo cerrado cada arista la comparten exactamente dos triángulos.

**area check** — una triangulación correcta de una cara plana cubre exactamente el área de
la cara; un abanico malo sobre una cara cóncava cubre **más** (sus triángulos se solapan y
se salen).

---

## Cómo funciona realmente

### 1. Dónde vive

`engine/src/triangulate.rs`, al que llama el parser de `.obj` para cada línea de cara. Es
código del engine (ft_newton y ft_vox también cargan `.obj`) y no sabe nada de scop.

```rust
pub(crate) fn triangulate(positions: &[Vec3], corners: &[u32]) -> Vec<[u32; 3]>;
```

Tres esquinas vuelven sin cambios. Todo lo demás pasa por los cuatro pasos de abajo.

### 2. La normal de Newell

```
n = (0, 0, 0)
para cada arista a → b de la cara (incluida la última → la primera):
    n.x += (a.y − b.y) × (a.z + b.z)
    n.y += (a.z − b.z) × (a.x + b.x)
    n.z += (a.x − b.x) × (a.y + b.y)
```

Si `n` es (casi) cero, la cara no tiene área (todas las esquinas en una línea): abanico,
no hay nada que recortar.

### 3. Aplanar

```
quitar el eje donde |n| es mayor:
    x → quedarse (y, z)      y → quedarse (z, x)      z → quedarse (x, y)
esas parejas mantienen el orden dextrógiro, así que el contorno va en sentido antihorario
cuando esa componente de n es positiva; si es negativa, reflejar el contorno
```

Ahora "esquina hacia fuera" = "el camino gira a la izquierda" = `turn(a, b, c) > 0`.

### 4. Recortar orejas

```
remaining = todas las esquinas, en orden
mientras queden más de 3:
    buscar una oreja: una esquina b (con vecinas a, c) donde
        turn(a, b, c) > 0                                   // gira hacia fuera
        y ninguna otra esquina restante está dentro de a-b-c // nada en medio
    no se encuentra → abanico con las esquinas restantes, parar   // cara rota (Q-17.4)
    quedarse el triángulo (a, b, c); quitar b
quedarse las 3 últimas esquinas como último triángulo
```

**La búsqueda empieza en la esquina 1.** En una cara convexa todas las esquinas son orejas,
así que esto corta (c0, c1, c2), luego (c0, c2, c3), … — exactamente el abanico de § 12. Los
archivos que ya estaban bien (el logo de 42, las dos teteras) salen idénticos byte a byte;
solo cambian las caras cóncavas.

Coste: para cada oreja se prueban todas las demás esquinas — O(n²) por cara. Las caras
reales tienen unas pocas esquinas, así que no importa.

### 5. Contar aristas abiertas

```
para cada triángulo, para cada una de sus 3 aristas:
    clave = las *posiciones* de las dos esquinas, la menor primero   // el sentido da igual
    contar cuántos triángulos usan esa clave
aristas abiertas = claves usadas exactamente una vez
```

Dos detalles del checkpoint:

- **Posiciones, no índices**, para que dos partes modeladas por separado que se tocan
  exactamente en los mismos puntos cuenten como unidas.
- **−0.0 cuenta como 0.0.** `teapot.obj` escribe 86 coordenadas como `-0.000000`: el mismo
  punto, pero otro patrón de bits, así que una comparación ingenua ve dos esquinas
  distintas. Sumar `0.0` convierte −0.0 en 0.0 antes de comparar. Ese desajuste es también
  una causa probable de los "artefactos en los bordes" que muestran otros renderizadores con
  este archivo.

scop imprime el número al cargar un modelo:
`teapot.obj: … corners, … triangles, 160 open edges, …`.

### 6. El veredicto de la tetera

`teapot.obj` y `teapot2.obj` dan **160 aristas abiertas** cada una, la mayoría (~100)
alrededor del borde de la tapa: las superficies de la tapa y del cuerpo se modelaron por
separado y nunca se unieron. Los dos archivos se ven igual en scop, y la Vista Rápida de
macOS muestra la misma ranura. Así que el hueco está en el modelo, y scop dibuja exactamente
lo que describe el archivo — por los dos lados, con la profundidad correcta. Soldar las
esquinas que casi se tocan (a ~0.017) lo cerraría, pero eso es cambiar el modelo, no
dibujarlo.

### Decisión (2026-11-18, `DECISIONS.md`)

| Elección | Frente a | Por qué |
|---|---|---|
| **Ear clipping sobre el contorno aplanado según la normal de Newell**, con abanico de respaldo | Quedarse con el abanico | Correcto para toda cara simple, convexa o cóncava, plana o un poco doblada; nunca se cuelga |
| | Una librería de triangulación | Prohibida por el subject |
| **Búsqueda de orejas desde la esquina 1** | Desde la esquina 0 | Las caras convexas conservan exactamente los triángulos de § 12 |
| **Dibujar la tetera tal como está modelada** | Soldar esquinas que casi se tocan | La ranura está en el archivo (medido), y los dos archivos de tetera coinciden |
| **Aristas abiertas comparadas por posición, −0.0 = 0.0** | Por índice / bits en crudo | Las costuras y los ceros con signo no son huecos reales |

Arreglo extra encontrado durante el checkpoint: la variable `TEX` del Makefile chocaba con
la `TEX` integrada de make (el programa TeX), así que el argumento de textura ahora es
`TEXTURE`.

---

## Trampas conocidas

- **El ear clipping no dibuja nada, o la mitad equivocada** — el contorno va en sentido
  horario, así que "gira a la izquierda" significa "apunta hacia dentro". Refleja el
  contorno cuando la cara se ve por detrás (el signo de la componente de Newell).
- **Las caras cóncavas siguen mal en modelos inclinados** — la normal salió del producto
  vectorial de las dos primeras aristas, y la esquina entre ellas es reflex: la normal se
  invierte, y todos los tests también. Usa la de Newell (Q-17.3).
- **El programa se cuelga con un archivo de un evaluador** — una cara en pajarita, ninguna
  oreja, y el bucle sigue buscando. Recurre al abanico (Q-17.4).
- **Esquinas alineadas tomadas como orejas** — un "giro" recto (0) da un triángulo de área
  cero. Exige `turn > umbral diminuto`.
- **Los modelos convexos cambian al añadir ear clipping** — la búsqueda empezaba en la
  esquina 0, así que sale otro conjunto de triángulos (válido igualmente). Inofensivo, pero
  empezando en la 1 se conserva exactamente la salida anterior.
- **Número de aristas abiertas demasiado alto en la tetera** — −0.0 y 0.0 comparados como
  puntos distintos. Normaliza antes el signo del cero.
- **"La ranura de la tapa es un bug de scop"** — compruébalo con el número de aristas
  abiertas y el segundo archivo de tetera antes de tocar código de renderizado (Q-17.5).

---

## Checkpoint

**Cumple (subject):** bonus — caras cóncavas y no coplanares; la tetera original renderizada
sin artefactos *que vengan de scop*. Método: primero el gate, luego Claude lo escribe y te
lo explica.

1. **Engine:** `engine/src/triangulate.rs` — normal de Newell → aplanar (reflejado cuando
   se ve por detrás) → ear clipping con la búsqueda desde la esquina 1 → abanico de
   respaldo. Lo llama el parser para cada cara.
2. **Engine:** `ObjModel::open_edges()`, comparando posiciones con −0.0 = 0.0.
3. **scop:** imprime el número de aristas abiertas junto con los de esquinas y triángulos.
4. **Archivo de autoevaluación:** `resources/test/concave.obj` — un prisma en forma de L
   (tapas de 6 esquinas) y un prisma de estrella de 5 puntas (tapas de 10 esquinas); 0
   aristas abiertas.
5. **Tests:** comprobaciones de área en la L y la estrella (las orejas cubren exactamente
   el contorno; el abanico cubre más), versiones en sentido horario e inclinadas, caras
   convexas iguales al abanico antiguo, un quad doblado da 2 triángulos, una pajarita y una
   línea recta nunca se cuelgan, recuentos de aristas abiertas (incluido el cero con signo).

**Verificar (hecho el 2026-11-18):** `make run OBJ=resources/test/concave.obj` — tapas de la
L y de la estrella limpias, sin cuñas cruzando las muescas; las dos teteras se ven igual, 160
aristas abiertas cada una; el logo de 42 sin cambios; `cargo test`,
`cargo clippy --all-targets` limpios; validación callada.

---

## Preguntas

Q-17.1 (recuerdo) ¿Qué es una oreja, y por qué cortar orejas de una en una siempre acaba en
una triangulación correcta?

Q-17.2 (aplicación) Sin mirar la pantalla, ¿cómo puedes comprobar que una triangulación de
una cara plana es correcta? ¿Qué dice esa comprobación de un abanico desde la esquina
equivocada de una L?

Q-17.3 (aplicación) ¿Por qué los tests de oreja se hacen sobre un contorno 2D aplanado, y
por qué scop usa la normal de Newell en vez del producto vectorial de las dos primeras
aristas de la cara?

Q-17.4 (predicción) Un archivo contiene una cara en pajarita (sus aristas se cruzan), así
que ninguna esquina es oreja. ¿Qué hace scop, y por qué es mejor que rechazar el archivo?

Q-17.5 (aplicación) Un evaluador señala la ranura alrededor de la tapa de la tetera y dice
que es un bug de scop. ¿Cómo demuestras de dónde viene de verdad la ranura?

Respuestas: `ANSWERS.md` § 17
