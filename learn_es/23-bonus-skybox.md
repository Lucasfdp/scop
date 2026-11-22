---
section: 23
slug: bonus-skybox
tier: T1
status: done
prereqs: [11, 14, 20, 22]
terms: [cube map, cube face, direction lookup, skybox, depth writing off, environment map, reflection, chrome, generated sky]
---

# Sección 23 — Un cielo alrededor de la escena (bonus)

**Profundidad: T1, sin matemáticas (como §§ 20 y 22).** Qué hace cada pieza, dónde vive y cómo
ver que está bien; las pocas fórmulas son funciones integradas de GLSL o vienen dadas en el
código.
Extra del bonus (elegido por el propietario, 2026-11-18): **un skybox con cube map, más un modo
cromado (reflectante)**.
**Serás capaz de:** rodear el modelo con un cielo que gira al orbitar, y convertir el modelo en
un espejo que lo refleja.

---

## La idea

**Una imagen que lo rodea todo.** Una textura normal (§ 14) es una sola imagen plana, que se
consulta con (u, v). Un **cube map** son seis imágenes cuadradas — las caras interiores de una
caja: derecha, izquierda, arriba, abajo, delante, detrás — guardadas como **una** textura. No se
consulta con (u, v) sino con una **dirección**: "¿qué hay por ahí?". La GPU elige la cara hacia
la que apunta esa dirección y el punto dentro de ella. Cualquier dirección tiene respuesta, así
que cubre todo el entorno.

**El skybox.** Dibuja una caja grande alrededor de la cámara, y colorea cada píxel de ella
consultando el cube map en la dirección de la cámara a ese píxel. La caja se mueve *con* la
cámara, así que nunca puedes acercarte a una pared ni ver una esquina; solo girar (la órbita)
cambia lo que ves. Eso es lo que hace que un cielo parezca infinitamente lejano (Q-23.2).

Se dibuja **primero**, detrás de todo, con la **escritura de profundidad desactivada**: pinta el
fondo pero deja la imagen de profundidad (§ 11) en "aquí aún no hay nada", así que el modelo
siempre se dibuja encima, sea la caja grande o pequeña (Q-23.3).

**Cromado.** Un espejo muestra lo que hay en la dirección en que la vista *rebota* en él. Para
cada píxel del modelo: la dirección de la cámara a ese punto, rebotada en la cara (su normal,
§ 20) — GLSL lo trae integrado: `reflect` — y luego la misma consulta al cube map. El modelo
ahora muestra el cielo que lo rodea, y cambia al orbitar o al girarlo. Las caras de scop son
planas (§ 20), así que cada cara es un espejo plano: saltos nítidos en las aristas, una tetera
bola de discoteca (Q-23.4).

**Qué cielo.** Sin descargas ni formatos de imagen nuevos: scop **genera** su cielo al
arrancar — un degradado azul arriba, un horizonte pálido, suelo más oscuro y un sol colocado
exactamente donde viene la luz de § 20, para que el sombreado y el cielo coincidan (Q-23.5). Si
hay seis caras `.tga` en `resources/sky/`, scop usa esas en su lugar.

**Dónde deja de valer la analogía:** un espejo de verdad también refleja el modelo (el pitorro
de la tetera se vería en su cuerpo). Un reflejo con cube map solo muestra el *cielo*: el modelo
nunca se ve a sí mismo.

---

## Los nombres

**cube map** — seis imágenes cuadradas, una por cara interior de una caja, guardadas como una
textura y consultadas con una dirección.

**cube face** — una de las seis: +x (derecha), −x (izquierda), +y (arriba), −y (abajo), +z, −z.
Vulkan las guarda como 6 capas en ese orden.

**direction lookup** — muestrear un cube map con una dirección (`samplerCube`): la GPU elige la
cara y el punto. La longitud de la dirección da igual.

**skybox** — una caja grande dibujada alrededor de la cámara, que se mueve con ella, coloreada a
partir del cube map: el cielo.

**depth writing off** — un ajuste de pipeline: comprobar contra la imagen de profundidad pero no
escribir en ella. El cielo pinta el fondo sin tapar nada.

**environment map** — un cube map usado para lo que rodea la escena (cielo y reflejos). scop
enlaza uno para cada draw (set 0).

**reflection** — la dirección de la vista rebotada en una superficie (`reflect` en GLSL) — hacia
donde "mira" un espejo.

**chrome** — modo de color 4 (C): cada píxel muestra el cielo en su dirección de reflejo.

**generated sky** — el cube map incorporado de scop, calculado al arrancar: degradado,
horizonte, suelo, sol.

---

## Cómo funciona realmente

### 1. Engine: cube maps, el entorno, un pipeline de fondo

```
Gfx::create_cube_texture([&Image; 6])     // 6 caras cuadradas, mismo tamaño, orden +x −x +y −y +z −z
    una imagen, 6 capas, "cube compatible"; tipo de view CUBE; el mismo sampler que § 14
set 0 binding 2: el entorno (samplerCube), cualquier draw puede leerlo
    Gfx::set_environment(cube)             // espera a la GPU, hace que cada slot apunte a él
    por defecto: un cubo blanco de 1×1, así siempre es válido
uniform de la cámara: + la posición de la cámara (vec4)   // 80 → 96 bytes; el cromado la necesita
ajuste de pipeline: escritura de profundidad desactivada  // Gfx::create_background_pipeline
```

### 2. scop: el cielo

```
cielo generado: para cada una de las 6 caras, cada texel → su dirección → un color
               (degradado según lo alto que apunte; sol donde viene la luz)
               — o resources/sky/{px,nx,py,ny,pz,nz}.tga si están las seis
mesh del cielo: un cubo, dibujado primero con el pipeline de fondo
sky.vert:      caja centrada en la cámara (model = mover a la cámara × hacerla grande)
               dirección = la propia posición de la esquina (la caja está centrada en la cámara)
sky.frag:      color = consulta al entorno en esa dirección
E: cielo activado / desactivado (desactivado → el fondo liso, como antes)
```

### 3. scop: cromado

```
mesh.vert: también pasa la posición de la esquina en el mundo (model × position)
mesh.frag: modo 4 (C):  view = posición en el mundo − posición de la cámara
                        bounce = reflect(view, normal)
                        color = consulta al entorno en `bounce`
           sin luz (el cielo *es* su iluminación), como orientación / profundidad (§ 19)
recarga en caliente: vigila también sky.vert / sky.frag
```

### Decisiones

| Elección | Frente a | Por qué |
|---|---|---|
| **Caja centrada en la cámara, dibujada primero, sin escritura de profundidad** | Un triángulo de pantalla completa que calcula la dirección de cada píxel | Sin matemáticas nuevas: las propias esquinas de la caja son las direcciones; reutiliza el bloque de push y la cámara |
| **Cielo generado, seis `.tga` opcionales** | Descargar un cielo (PNG/HDR) | Solo nuestro cargador TGA; nada que buscar ni licenciar; el sol coincide con la luz |
| **Entorno en el set 0 (por fotograma, en cada draw)** | Una segunda textura en el set 1 | El cromado necesita a la vez la imagen *y* el cielo; el set 1 sigue siendo "la imagen de este draw" |
| **Posición de la cámara en el uniform** | Calcularla en el shader a partir de la matriz de vista | Una línea en el bloque de la cámara; sin inversa de matriz |
| **Cromado como modo de color 4 (C)** | Una tecla aparte | Es otra forma de colorear el modelo, como orientación y profundidad |
| **Se mantienen las normales planas** | Normales suaves para el cromado | La misma mesh que todo lo demás; los espejos facetados son fieles a estos archivos |

---

## Trampas conocidas

- **Sol en el lado equivocado, o cielo boca abajo / reflejado** — las seis caras en el orden
  equivocado, o la imagen de una cara invertida. El orden de Vulkan es +x, −x, +y, −y, +z, −z,
  cada una con su propio "arriba". Comprobación: con B activada, el lado iluminado del modelo
  debería mirar al sol.
- **Modelo invisible detrás del cielo** — el pipeline del cielo escribe profundidad (Q-23.3), o
  se dibuja *después* del modelo con el test de profundidad desactivado.
- **Cielo recortado en las esquinas** — la caja llega más allá del plano lejano (§ 11):
  mantenla dentro.
- **Error de validación: tipo de image view** — una view de cubo necesita una imagen creada
  "cube compatible" con 6 capas, y caras cuadradas de un mismo tamaño.
- **El cromado parece la luz, no el cielo** — al modo 4 aún se le aplica el código de la luz; o
  el entorno no está enlazado (se ve el blanco por defecto: todo blanco).
- **El reflejo no cambia al orbitar** — la posición de la cámara no se actualiza, o `view` sale
  de un punto fijo.
- **La recarga en caliente ignora el cielo** — la vigilancia solo conocía `mesh.*`; ahora vigila
  los cuatro shaders.

---

## Checkpoint

**Cumple (subject):** extra del bonus (skybox, cromado). Método: primero el gate, luego Claude lo
escribe y te lo explica.

1. **Engine:** `create_cube_texture`; entorno en el binding 2 del set 0 + `set_environment` +
   blanco por defecto; bloque de la cámara + posición (96 bytes); `create_background_pipeline`
   (sin escritura de profundidad).
2. **scop:** cielo generado (y la opción de seis `.tga`); mesh del cielo, `sky.vert`/`sky.frag`,
   dibujado primero; E lo alterna; cromado como 4º modo de C; posición en el mundo desde
   `mesh.vert`; la recarga en caliente vigila los cuatro shaders; controles, README.
3. **Tests:** las caras del cubo deben ser cuadradas y del mismo tamaño; el cielo generado es
   claro arriba, oscuro abajo y más brillante hacia el sol (en la dirección de la luz); bytes de
   la cámara 96 con la posición tras la luz; la caja del cielo cabe dentro del plano lejano.

**Verificar:** E — cielo alrededor del logo, que gira al orbitar; B — el lado iluminado mira al
sol; C hasta cromado — el logo refleja el cielo, que cambia al orbitar; tetera cromada — una bola
de discoteca; multitud (G) en cromado; E desactivado — el fondo liso, como antes; validación
callada.

---

## Preguntas

Q-23.1 (recuerdo) ¿Qué es un cube map, y por qué se consulta con una dirección en vez de con
(u, v)?

Q-23.2 (aplicación) La caja del cielo se mueve con la cámara. ¿Qué verías al orbitar si se
quedara donde estaba?

Q-23.3 (predicción) Haces la caja del cielo pequeña — 1 unidad de lado, aún centrada en la
cámara — y la dibujas primero con la escritura de profundidad **activada**. ¿Qué ves? ¿Por qué lo
arregla desactivar la escritura de profundidad?

Q-23.4 (predicción) En modo cromado, ¿por qué el logo de 42 muestra saltos bruscos en el reflejo
en las aristas entre caras, y la tetera parece una bola de discoteca?

Q-23.5 (aplicación) ¿Por qué colocar el sol generado exactamente donde viene la luz de § 20? ¿Qué
se vería mal si estuviera en otro sitio?

Respuestas: `ANSWERS.md` § 23
