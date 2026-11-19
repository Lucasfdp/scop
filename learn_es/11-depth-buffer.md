---
section: 11
slug: depth-buffer
tier: T2
status: done
prereqs: [10]
terms: [occlusion, painter's algorithm, depth test, depth buffer, depth value, compare op, depth write, depth clear value, depth image, depth format, format features, image aspect, depth attachment, depth layout, store op DONT_CARE, early and late fragment tests, stencil, z-fighting, back-face culling, winding order]
---

# Sección 11 — Qué está delante de qué

**Profundidad: T2** — todo lo 3D que dibujes en tu vida necesita esto, y cuando está mal
el modelo se ve del revés sin ningún mensaje de error. Deberías saber decir qué guarda el
depth buffer, por qué cada ajuste tiene el valor que tiene, y diagnosticar los fallos
clásicos a partir de una captura.
**Serás capaz de:** dibujar el cubo (y luego el logo de 42 y la tetera) con las caras
cercanas tapando siempre a las lejanas, en cualquier ángulo.

---

## La idea

Mira tu cubo de § 10 unos segundos: en algunos ángulos una cara de *detrás* se ve encima de
una cara delantera. A las matrices no les pasa nada. La GPU simplemente dibuja los
triángulos **en el orden de la lista de índices**, y cada triángulo nuevo pinta encima de
los píxeles que ya haya. Gana el que se dibuja **el último**, esté cerca o lejos.

El arreglo obvio es pintar como un pintor: ordenar los triángulos de lejos a cerca y
dibujar primero los lejanos, para que los cercanos los tapen. Funciona a medias, pero cuesta
una ordenación cada fotograma (el cubo gira, así que el orden no para de cambiar), y **no
puede** con triángulos que se atraviesan entre sí: cada uno está en parte delante del otro,
así que ningún orden es correcto.

El arreglo que usan todas las GPUs es mucho más sencillo: **recordar, para cada píxel, a
qué distancia está lo que hay dibujado ahora mismo en él.** Cuando un triángulo nuevo quiere
pintar un píxel, se compara:

- el nuevo está **más cerca** que lo que hay → se pinta, y se recuerda la nueva distancia;
- el nuevo está **más lejos** → se tira, el píxel se queda con lo que tenía.

Ahora el orden en que se dibujan los triángulos ya no importa. Imagina una hoja de papel
del tamaño de la ventana en la que, a lápiz, has escrito una distancia en cada cuadradito.
Antes de cada fotograma lo borras todo y escribes "lo más lejos posible" en todas partes,
así que lo primero que se dibuja en cualquier cuadradito siempre le gana a "nada".

El número de distancia ya lo conoces: es la **profundidad** de la proyección de § 9, donde
el plano cercano sale como 0 y el lejano como 1. "Lo más lejos posible" es 1.

**Dónde deja de valer la analogía:** la hoja no la comprueba tu código en absoluto. Activas
la comprobación en el pipeline, le das a la GPU una imagen donde guardar los números, y la
GPU hace la comparación ella sola, para cada píxel de cada triángulo, en hardware.

---

## Los nombres

**occlusion** — una cosa que tapa a otra a la vista. "Oclusión correcta" = lo cercano tapa
lo lejano.

**painter's algorithm** — dibujar de lejos a cerca para que lo cercano pinte encima de lo
lejano. Se llama así por cómo los pintores hacen primero el fondo. Falla con triángulos que
se cruzan y hay que reordenar cada vez que algo se mueve. No se usa en scop.

**depth test** — la comparación por píxel: "¿este fragmento nuevo está más cerca que lo que
hay?". Un *fragmento* (§ 6) es un píxel candidato producido por un triángulo. Si falla → el
fragmento se tira antes de escribir ningún color.

**depth buffer** — la "hoja de papel": un número por píxel, la profundidad de lo que haya
dibujado ahí en ese momento. También llamado **z-buffer**, porque la profundidad sale de z.

**depth value** — el número guardado: el `z / w` de § 9 tras la división de perspectiva. 0
en el plano cercano, 1 en el lejano.

**compare op** — `VkCompareOp`, "operación de comparación": la regla de "más cerca". scop
usa `LESS`: el fragmento nuevo pasa si su profundidad es **menor que** la guardada (más
pequeño = más cerca).

**depth write** — tras pasar un fragmento, guardar su profundidad en el buffer. Un
interruptor distinto del test: puedes comprobar sin escribir (se usa para cosas
translúcidas, más adelante).

**depth clear value** — a qué se reinicia cada píxel al principio de cada fotograma:
**1.0**, el plano lejano, para que el primer fragmento en cualquier sitio pase siempre
`LESS`.

**depth image** — el depth buffer como objeto de Vulkan: un `VkImage` del tamaño de las
imágenes de la swapchain, en memoria `DEVICE_LOCAL` (solo lo toca la GPU), más un
`VkImageView` (§ 4) para que el renderizado pueda usarlo.

**depth format** — cómo se guarda cada número de profundidad. scop pide `D32_SFLOAT`:
**D**epth (profundidad), **32** bits, **S**igned **FLOAT**. La misma idea que el
`R32G32B32_SFLOAT` de § 10, con **D** de profundidad en vez de canales de color.

**format features** — lo que una GPU puede hacer con un formato, se pregunta con
`get_physical_device_format_properties`. Necesitas la feature `DEPTH_STENCIL_ATTACHMENT`
("usable como attachment de profundidad/stencil") con tiling óptimo, o el formato no está
permitido. scop comprueba una lista corta y se queda con el primero soportado.

**image aspect** — de qué parte de una imagen habla una view o una barrera: `COLOR` para
imágenes de color (el `COLOUR_RANGE` de § 5), `DEPTH` para imágenes de profundidad. Usar
`COLOR` en una imagen de profundidad es un error de validación.

**depth attachment** — la imagen de profundidad enchufada al renderizado. Con dynamic
rendering (decisión de § 6) es un `RenderingAttachmentInfo` más, pasado como
`depth_attachment` junto al colour attachment.

**depth layout** — el layout (§ 5) en el que debe estar una imagen para usarse como depth
attachment: `DEPTH_STENCIL_ATTACHMENT_OPTIMAL`. Una barrera por fotograma la lleva ahí, como
la barrera de la imagen de color.

**store op DONT_CARE** — "me da igual", como store op (§ 6): tras renderizar, la GPU puede
tirar los números de profundidad. Nadie los lee después del fotograma, así que guardarlos
solo costaría ancho de banda de memoria. La imagen de color usa `STORE`, porque se muestra.

**early and late fragment tests** — las dos etapas del pipeline donde puede hacerse el test
de profundidad: *early* (temprana) = antes de que se ejecute el fragment shader (rápido: los
fragmentos ocultos se saltan el shader por completo), *late* (tardía) = después. Una
barrera sobre la imagen de profundidad nombra las dos etapas.

**stencil** — un segundo número por píxel que llevan algunos formatos de profundidad (el
`S8` de `D32_SFLOAT_S8_UINT`), usado para efectos de máscara como contornos y espejos. Se
llama así por las plantillas de pintura (stencil). scop no lo usa; solo explica por qué el
nombre del layout dice `DEPTH_STENCIL`.

**z-fighting** — dos superficies a casi la misma profundidad, de modo que el redondeo decide
quién gana píxel a píxel y fotograma a fotograma: un patrón de rayas que parpadea. Empeora
con un plano cercano diminuto (abajo).

**back-face culling** — saltarse los triángulos que miran en dirección contraria a la
cámara, antes de dibujarlos. Una optimización, **no** un sustituto del depth buffer (una
cara cercana puede seguir tapando otra cara que mira hacia delante). Desactivado en scop —
ver la decisión.

**winding order** — si las esquinas de un triángulo van en sentido horario o antihorario
vistas en pantalla. El culling lo usa para decidir "me mira" vs "mira hacia el otro lado".

---

## Cómo funciona realmente

### 1. La imagen de profundidad

Al arrancar, junto a la swapchain (debe ser del mismo tamaño que las imágenes de la
swapchain):

```
1. elegir formato:    el primero de [D32_SFLOAT, D32_SFLOAT_S8_UINT, D24_UNORM_S8_UINT]
                      cuyas features con tiling óptimo incluyan DEPTH_STENCIL_ATTACHMENT
2. create_image:      2D, extent de la swapchain, ese formato, uso DEPTH_STENCIL_ATTACHMENT
3. memoria:           requirements → tipo DEVICE_LOCAL → allocate → bind (igual que el Buffer de § 7)
4. create_image_view: aspect DEPTH
```

Es la versión imagen del `Buffer` de § 7: objeto, memoria, bind. Una imagen de
profundidad, no una por slot de fotograma — ver la decisión y el § 3 de abajo.

Cuando la ventana se redimensiona, la imagen de profundidad debe reconstruirse junto con la
swapchain (§ 16).

### 2. El pipeline la activa

Dos añadidos a la creación del pipeline:

```
depth/stencil state:  depth_test_enable  = true
                      depth_write_enable = true
                      depth_compare_op   = LESS
rendering info:       depth_attachment_format = el formato elegido
```

La segunda línea es la versión de dynamic rendering de "este pipeline dibuja en una imagen
de profundidad de este formato". Debe coincidir con la imagen que enchufas de verdad, o la
validación se queja.

### 3. Cada fotograma

```
barrera:  imagen de profundidad  UNDEFINED → DEPTH_STENCIL_ATTACHMENT_OPTIMAL
          esperar a: las escrituras de profundidad del fotograma anterior (LATE_FRAGMENT_TESTS, DEPTH…_WRITE)
          antes de:  los tests de profundidad de este fotograma (EARLY | LATE_FRAGMENT_TESTS, READ | WRITE)
begin rendering:
          colour attachment  (sin cambios)
          depth attachment   view = view de profundidad, layout DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                             load CLEAR a 1.0, store DONT_CARE
… draws sin cambios …
```

Por qué `UNDEFINED` como layout antiguo: "me da igual lo que tenga", porque se va a limpiar
de todos modos. El mismo truco que con la imagen de color (§ 5).

Por qué la barrera espera a las escrituras de profundidad del fotograma *anterior*: con dos
fotogramas en vuelo, la GPU podría estar aún terminando los tests de profundidad del
fotograma N cuando el N+1 empieza a usar la misma imagen. La barrera hace que el trabajo de
profundidad de N+1 espere al de N. Los dos fotogramas van en la misma cola, así que aquí
basta con una barrera; no hace falta semáforo (ver la chuleta de sincronización: "los
comandos posteriores esperan a los anteriores").

### 4. Qué hace la GPU por fragmento

```
fragmento en el píxel (x, y) con profundidad d
   ¿d < depth_buffer[x, y]?      (compare op LESS)
     sí → ejecutar/quedarse el color, depth_buffer[x, y] = d     (depth write)
     no → descartar
```

Cuando puede, la GPU hace esto **antes** del fragment shader (tests tempranos), así que los
píxeles ocultos no cuestan casi nada.

### 5. Precisión, en un párrafo

La profundidad tras la división de perspectiva **no** se reparte uniformemente: la mayor
parte del rango 0..1 se gasta cerca del plano cercano. Acercar el plano cercano
(0.1 → 0.0001) aprieta el resto de la escena en una franja diminuta de números, y las
superficies lejanas empiezan a hacer z-fighting. Regla práctica: deja `near` tan grande
como puedas permitirte. El 0.1 de scop está bien.

### Decisión: una imagen de profundidad + una barrera; culling desactivado

| Opción | A favor | En contra |
|---|---|---|
| **Una imagen de profundidad, barrera cada fotograma** ✅ | Lo estándar; la mitad de memoria; la barrera es una más de lo que ya hace § 5 | Los fotogramas en vuelo se serializan en la profundidad (coste mínimo; van en una cola igualmente) |
| Una imagen de profundidad por slot | Sin barrera entre fotogramas | El doble de memoria, más objetos que reconstruir al redimensionar (§ 16), sin ganancia visible |
| **Back-face culling desactivado** ✅ | Correcto para cualquier `.obj`, tenga el winding que tenga | Dibuja también las caras traseras ocultas (el test de profundidad las sigue ocultando) |
| Back-face culling activado | Hasta ~2× menos fragmentos | Los `.obj` no garantizan un winding coherente; un modelo con winding mezclado pierde caras. Revisar para ft_vox, donde las meshes las haces tú |

---

## Trampas conocidas

- **Las caras traseras se siguen viendo, sin error** — la imagen de profundidad está
  enchufada pero el `depth_test_enable` del pipeline está apagado, o lo está
  `depth_write_enable` (Q-11.2).
- **Solo el fondo, nada dibujado** — compare op `GREATER` (o clear value 0.0 con `LESS`):
  nada puede ganarle al valor de limpieza (Q-11.3).
- **Error de validación al dibujar sobre el formato del depth attachment** — el
  `depth_attachment_format` del pipeline no coincide con el formato de la imagen enchufada
  (o falta).
- **Error de validación sobre el aspect de la imagen** — una view o una barrera sobre la
  imagen de profundidad usa `COLOR` en vez de `DEPTH`.
- **Error de validación sobre el layout** — falta la barrera, o el renderizado nombra un
  layout distinto del que puso la barrera.
- **Rayas que parpadean donde dos caras casi se tocan** — z-fighting. Comprueba que el plano
  cercano no sea diminuto (Q-11.5).
- **Todo el modelo dibujado del revés (ves el interior de las caras lejanas)** — el test de
  profundidad está bien; el back-face culling está activado con el `front_face`
  equivocado. Deja el culling apagado.
- **Error de validación al apagar: imagen / image view / memoria sin destruir** — los tres
  objetos de la imagen de profundidad van después de `device_wait_idle`, antes del
  dispositivo.

---

## Checkpoint

**Cumple (subject):** un objeto 3D dibujado correctamente desde cualquier ángulo —
necesario antes de que aparezca el logo en § 12. Método: primero el gate, luego Claude lo
escribe y te lo explica.

1. **Engine — imagen de profundidad:** un pequeño `DepthImage` (imagen + memoria + view),
   creado junto con la swapchain, formato elegido de la lista según sus features,
   `DEVICE_LOCAL`, destruido en el orden correcto.
2. **Engine — pipeline:** test + escritura de profundidad activados, `LESS`, y el formato
   de profundidad en el create info de rendering.
3. **Engine — cada fotograma:** la barrera de profundidad, y luego un depth attachment
   limpiado a 1.0 con store `DONT_CARE`.
4. **scop:** sin cambios — el mismo cubo girando.

**Verificar:**
- `cargo run -p scop -- resources/42.obj`: el cubo nunca muestra una cara trasera a través
  de una delantera, en ningún ángulo.
- Validation layer callada, también al salir.
- Experimento: pon la compare op en `GREATER` → solo el fondo. Vuelve a dejarla.
- `cargo test`, `cargo clippy --all-targets` limpios.

---

## Preguntas

Q-11.1 (recuerdo) ¿Qué guarda el depth buffer para cada píxel, a qué se reinicia al
principio de cada fotograma, y por qué ese valor?

Q-11.2 (predicción) El test de profundidad está activado pero **la escritura de
profundidad está desactivada**. Nada más cambia. ¿Qué ves en el cubo que gira, y por qué?

Q-11.3 (predicción) Pones la compare op en `GREATER` y sigues limpiando a 1.0. ¿Qué ves, y
por qué?

Q-11.4 (aplicación) La store op de la imagen de color es `STORE`, pero la de la imagen de
profundidad es `DONT_CARE`. ¿Por qué no pasa nada por tirar los números de profundidad al
final del fotograma?

Q-11.5 (predicción) Para dejar que la cámara se acerque mucho al logo, cambias `near` de
0.1 a 0.0001. ¿Qué podría salir mal, y en qué parte de la pantalla lo notarías primero?

Respuestas: `ANSWERS.md` § 11
