---
section: 18
slug: bonus-texture-mapping
tier: T1
status: done
prereqs: [17]
terms: [box mapping, texel density, texture seam, triplanar mapping, object line, per-object bounding box, one draw per object, polygon mode, fillModeNonSolid, portability subset, point list, gl_PointSize, zoom vs dolly, scroll delta]
---

# Sección 18 — Mejor texturizado, modos de vista y zoom (bonus)

**Profundidad: T1** — sobre todo reutilización: las normales de § 17, el
descriptor-por-textura de § 14, los ajustes de pipeline de § 6. Bonus (5 pts): *"¡Aplica las
texturas de forma más sutil, evitando el estiramiento visible en ciertas caras!"*, más tres
extras de tu elección que listará § 19: **una imagen por objeto**, **sólido → wireframe →
puntos**, y **zoom**.
**Serás capaz de:** texturizar cualquier modelo sin rayas, darle a cada objeto de un archivo
su propia imagen, cambiar de modo de vista con una tecla y hacer zoom como con el objetivo
de una cámara.

---

## La idea

**Rayas.** Desde § 14 la imagen se proyecta en línea recta a través de todo el modelo, como
un proyector de diapositivas, a lo largo de un eje. Las caras que miran al proyector reciben
bien la imagen; las caras que van *a lo largo* del haz reciben una línea fina de la imagen
emborronada por encima.

El arreglo es usar **tres proyectores** en vez de uno — frontal, lateral y cenital — y darle a
cada cara el que tenga más de frente. Una cara que mira de lado recibe el proyector lateral,
una que mira hacia arriba el cenital. Ninguna cara queda nunca de canto respecto a su
proyector, así que nada se raya. Ya sabes cómo saber hacia dónde mira una cara: la normal de
Newell de § 17.

Otra fuente de estiramiento: si una imagen se pone sobre una caja larga y delgada y se
aprieta para que encaje en las dos direcciones, se estira. Usar **la misma escala** en
horizontal y vertical mantiene cuadrados los cuadrados de la imagen.

**Una imagen por objeto.** Un archivo puede contener varios objetos (líneas `o nombre` —
`concave.obj` tiene dos). Trata cada uno como su propio modelo a efectos de textura: su
propia bounding box (para que su imagen se ajuste a *él*, no al archivo entero) y su propio
**draw** — porque qué imagen lee un draw se elige enlazando un set de textura *por draw*
(§ 14). Dale a scop varias imágenes y los objetos se turnan para usarlas; una tecla rota la
asignación.

**Modos de vista.** La GPU puede rellenar triángulos (lo que hemos hecho), dibujar solo sus
aristas (wireframe) o dibujar solo sus esquinas (puntos). Cada uno es un pipeline distinto
(§ 6) — solo un ajuste distinto — así que cambiar es "dibujar con el pipeline A, B o C este
fotograma".

**Zoom.** Hay dos formas de hacer el modelo más grande en pantalla. *Dolly*: acercarse (tus
W/S). *Zoom*: mantener la distancia y cerrar el objetivo (el campo de visión, § 9). Se ven
distinto: acercarse exagera la perspectiva (el lado cercano se hincha); hacer zoom la aplana,
como un teleobjetivo. scop gana un zoom de verdad, en la rueda del ratón (y teclas, para
portátiles).

**Dónde deja de valer la analogía:** tres proyectores se encuentran en costuras — donde dos
caras vecinas eligieron proyectores distintos, la imagen no continúa suavemente por la
arista. Ese es el coste honesto de este método (ver triplanar mapping abajo).

---

## Los nombres

**box mapping** — elegir, por cara, una de tres proyecciones (a lo largo de x, y o z) según
la normal de la cara: el eje hacia el que más mira la cara. También llamado mapeo cúbico o
planar en tres ejes.

**texel density** — cuántos texels (§ 14) caen en cada unidad de superficie. Misma densidad
en las dos direcciones = sin estiramiento: escalar u y v por **un** número, no cada uno por
su propio lado de la caja.

**texture seam** — una arista donde la imagen salta, porque las caras a cada lado usan
proyecciones distintas. El box mapping siempre tiene alguna; quedan donde el modelo dobla
una esquina.

**triplanar mapping** — el primo suave: el fragment shader muestrea las tres proyecciones y
las mezcla según la dirección de la superficie, escondiendo las costuras. Cuesta 3 consultas
por píxel y necesita una normal por píxel; apuntado para ft_vox, no se hace aquí.

**object line** — `o nombre` en un `.obj`: "las caras de abajo pertenecen a un objeto
nuevo". (`g nombre`, un *grupo*, es parecido; scop trata ambos como límites de objeto.)
Saltado hasta ahora (§ 12).

**per-object bounding box** — el mín/máx solo de las esquinas de ese objeto: el marco al que
se ajusta su imagen.

**one draw per object** — cada objeto se convierte en su propia mesh (`MeshId`) y su propia
llamada a `draw_textured`, para que cada uno pueda enlazar un set de textura distinto.

**polygon mode** — el ajuste de pipeline de cómo se rasterizan los triángulos: `FILL`
(sólido), `LINE` (solo aristas), `POINT` (solo esquinas).

**fillModeNonSolid** — la feature de dispositivo ("fill mode non-solid", relleno no sólido)
que necesitan `LINE` y `POINT`. Hay que *pedirla* al crear el dispositivo (§ 3) — soportada
no es lo mismo que activada.

**portability subset** — la lista de cosas que no puede hacer una capa de
Vulkan-sobre-otra-cosa como MoltenVK (Vulkan sobre Metal). Una de ellas: `pointPolygons` —
Metal no tiene modo de "dibujar triángulos como puntos", así que `POLYGON_MODE_POINT` está
vetado en tu Mac.

**point list** — la forma de esquivarlo: un pipeline cuya *topología* (§ 6) es
`POINT_LIST`, así que cada vértice se dibuja como un punto. Misma mesh, misma llamada de
draw, otro pipeline.

**gl_PointSize** — lo que escribe el vertex shader para decir cuánto mide cada punto, en
píxeles. Se ignora con triángulos, es obligatorio con puntos.

**zoom vs dolly** — zoom: cerrar el campo de visión, la cámara se queda quieta (la
perspectiva se aplana). Dolly: mover la cámara (o el modelo) más cerca (la perspectiva se
exagera). Términos de cine.

**scroll delta** — cuánto se movió la rueda del ratón en este fotograma. Los ratones
reportan "líneas", los trackpads "píxeles"; el engine convierte ambos en un solo número.

---

## Cómo funciona realmente

### 1. Box mapping (scop)

```
por cara del .obj:
    normal = Newell(cara)                         // § 17
    axis   = aquel en el que |normal| es mayor    // x, y o z
    quedarse con las otras dos coordenadas como (u, v):
        mira según x → (z, y)    según y → (x, z)    según z → (x, y)
    invertir u cuando la cara mira en sentido "negativo", para que las imágenes no salgan reflejadas
    v = 1 − v  en las vistas laterales, para que "arriba" siga arriba en la imagen
escala: u, v ÷ el lado más largo de la caja del objeto    (un número → sin estiramiento)
        u × alto de la imagen ÷ ancho                     (para que una imagen ancha no se aplaste)
```

Cada esquina ya estaba duplicada por triángulo (§ 13), así que dos caras que comparten una
esquina pueden darle UVs distintas — justo lo que necesita el box mapping.

### 2. Objetos (parser del engine + scop)

```
parser: "o nombre" / "g nombre" → empezar un objeto nuevo; los triángulos recuerdan su objeto
        (ObjModel::triangle_objects, object_names; sin líneas o/g → un objeto)
scop:   por objeto: su bounding box, su mesh (solo sus triángulos), su imagen
        imágenes: scop model.obj [a.tga b.tga …] → el objeto i usa la imagen (i + shift) mod count
        N: shift += 1  (rotar quién recibe qué imagen)
```

### 3. Modos de vista (engine)

```
sólido    → polygon mode FILL, topología TRIANGLE_LIST                 (como ahora)
wireframe → polygon mode LINE, topología TRIANGLE_LIST, necesita fillModeNonSolid activada
puntos    → polygon mode FILL, topología POINT_LIST, el vertex shader escribe gl_PointSize
```

El engine activa `fillModeNonSolid` al crear el dispositivo *si* la GPU la tiene, e informa
de qué estilos puede hacer; la tecla V de scop va pasando por ellos. El test de profundidad
sigue activo en todos los modos; el culling sigue apagado, así que el wireframe muestra
también las aristas traseras (un aspecto de rayos X).

### 4. Zoom (entrada del engine + scop)

```
engine: WindowEvent::MouseWheel → Input::scroll() de este fotograma
scop:   fov_y = clamp(fov_y × 0.9^scroll, 10°, 100°)     // y teclas Z / X, por segundo
        R lo reinicia a 60°
```

Multiplicar (no sumar) hace que cada muesca de la rueda se sienta igual a cualquier nivel de
zoom.

### Decisiones

| Elección | Frente a | Por qué |
|---|---|---|
| **Box mapping por cara** | Triplanar en el shader (más suave, 3× consultas, necesita normales) | Sin rayas; reutiliza § 17; las costuras en las esquinas son aceptables |
| **Una escala para u y v**, con corrección del aspecto de la imagen | Ajustar cada lado de la caja por separado (§ 14) | Eso *era* una fuente de estiramiento |
| **`o` y `g` como objetos; imágenes asignadas por turnos, N rota** | Leer los nombres de textura del `.mtl` (`map_Kd`) | Funciona con cualquier archivo y cualquier imagen que pases; `.mtl` es material de ideas para § 19 |
| **Puntos vía `POINT_LIST`** | `POLYGON_MODE_POINT` | No disponible en Metal (portability subset `pointPolygons`) |
| **Wireframe vía `POLYGON_MODE_LINE`**, feature comprobada | Construir un index buffer de aristas | Un ajuste de pipeline; cae limpiamente a otra opción si la GPU no lo tiene |
| **Zoom = campo de visión**, rueda del ratón + teclas | Mover la cámara | El dolly ya existe (W/S); un zoom de verdad es lo nuevo |

---

## Trampas conocidas

- **Error de validación al crear el pipeline de wireframe** — `fillModeNonSolid` no
  *activada* al crear el dispositivo (soportada no basta) (Q-18.4).
- **El modo puntos no muestra nada, o puntos de 1 píxel** — `gl_PointSize` sin escribir, o el
  rango de tamaño de punto de la GPU es 1..1 (falta `largePoints`): comprueba
  `limits.point_size_range`.
- **Imagen reflejada en algunas caras** — falta la inversión de u en las caras que miran en
  sentido negativo.
- **La imagen sigue estirada en caras largas** — u y v escaladas por lados distintos de la
  caja.
- **Todos los objetos muestran una sola imagen** — sigue siendo una mesh / un draw; o el
  shift se aplica una vez en vez de por objeto.
- **Costuras donde las caras se encuentran en ángulo** — el coste honesto del box mapping;
  esperado.
- **El zoom salta, o invierte la imagen** — el campo de visión llega a 0° o 180°: limítalo.
- **El zoom con trackpad va demasiado rápido** — deltas en píxeles tratados como líneas;
  escálalos.

---

## Checkpoint

**Cumple (subject):** bonus — texturas sin estiramiento visible; más extras: una imagen por
objeto, modos de vista, zoom. Método: primero el gate, luego Claude lo escribe y te lo
explica.

1. **Engine:** `ObjModel::{triangle_objects, object_names}` a partir de las líneas `o`/`g`;
   estilos de pipeline (sólido, wireframe, puntos) con la comprobación de feature;
   `Input::scroll()`; el vertex shader escribe `gl_PointSize`.
2. **scop:** UVs con box mapping por cara con una escala y corrección de aspecto; una mesh
   + draw por objeto; imágenes desde la línea de comandos (la incrustada por defecto si no
   hay ninguna), N rota; V recorre los modos de vista; rueda del ratón / Z X para zoom; R
   reinicia el zoom; lista de controles actualizada; controles del README actualizados.
3. **Tests:** el box mapping elige el eje correcto y nunca raya (el área UV de una cara
   nunca es ~0); u y v comparten una escala; los objetos se separan bien; el zoom se
   limita.

**Verificar:** `make run OBJ=resources/test/concave.obj` con dos imágenes — cada forma su
propia imagen, N las intercambia; los lados delgados del logo de 42 muestran imagen, no
rayas; V recorre sólido → wireframe → puntos; la rueda hace zoom; validación callada.

---

## Preguntas

Q-18.1 (recuerdo) ¿Por qué la proyección de § 14 dejaba rayas en los lados delgados del
logo, y cómo las elimina el box mapping?

Q-18.2 (aplicación) ¿Por qué hay que dividir u y v entre el *mismo* número, en vez de cada
una entre su propio lado de la bounding box?

Q-18.3 (aplicación) Para darle a cada objeto su propia imagen, ¿por qué cada objeto necesita
su propia llamada de draw, en vez de una mesh grande dibujada una vez?

Q-18.4 (predicción) Creas el pipeline de wireframe con `POLYGON_MODE_LINE`, pero olvidaste
activar `fillModeNonSolid` al crear el dispositivo. ¿Qué pasa, y cuándo?

Q-18.5 (predicción) Haces el modelo el doble de grande en pantalla de dos formas: una
haciendo zoom (cerrando el campo de visión), otra acercándote. ¿Qué se ve distinto entre las
dos?

Respuestas: `ANSWERS.md` § 18
