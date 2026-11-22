---
section: 20
slug: bonus-camera-and-lighting
tier: T1
status: done
prereqs: [10, 13, 18, 19]
terms: [orbit camera, yaw and pitch, pitch clamp, look-at, mouse delta, directional light, normal, flat shading, diffuse, ambient, world-fixed light, normal transform]
---

# Sección 20 — Una cámara que manejas, y luz (bonus)

**Profundidad: T1, sin matemáticas (elección del propietario, 2026-11-19).** Planificada como
T2; se bajó porque el propietario ya ha hecho cámaras orbitales e iluminación (rubik, cubed).
Esta sección dice **qué hace cada pieza, dónde vive y cómo ver que está bien montada**. Las
fórmulas están en el código, con tests que las demuestran; aquí no se derivan.
**Serás capaz de:** orbitar alrededor del modelo con el ratón, iluminarlo con una luz tipo sol
y saber, mirando la pantalla, si la cámara, las normales y la luz están bien conectadas.

---

## La idea

**Dos formas de girar las cosas.** Hasta ahora giraba el *modelo* (teclas, giro automático) y
la cámara se quedaba quieta. Una **cámara orbital** hace lo contrario: el modelo se queda, la
cámara camina a su alrededor sobre una esfera, mirando siempre al centro del modelo. Arrastra
con el ratón: de lado rodeas (**yaw**), arriba/abajo pasas por encima o por debajo
(**pitch**). Las teclas siguen girando el modelo, así que las dos cosas funcionan juntas.

En pantalla las dos se ven idénticas — *hasta que hay luz*. A una luz fija en el mundo le da
igual dónde esté la cámara, así que al orbitar ves alternarse el lado iluminado y el oscuro.
Girar el modelo con las teclas cambia *qué caras* están iluminadas. Esa diferencia es la
comprobación más fácil de que las dos están bien montadas (Q-20.1).

**El pitch tiene un límite.** Pasa justo por encima y "arriba" deja de significar nada — la
cámara se da la vuelta de golpe en un fotograma. Así que el pitch para justo antes de la
vertical hacia arriba y hacia abajo.

**Una luz, como el sol.** Una **luz direccional** llega desde una dirección en todas partes,
desde infinitamente lejos. El brillo de una cara depende de lo de frente que mire a la luz:
de frente = brillo completo, de lado = tenue, de espaldas = solo el brillo **ambiente** (una
pequeña constante para que nada quede negro puro contra el fondo).

**Normales.** "Hacia dónde apunta una cara" — lo tienes desde § 17 (Newell) y § 18 (box
mapping). La iluminación lo necesita *por esquina*, enviado al shader como las UVs. Como cada
triángulo ya tiene sus propias esquinas (§ 13), las esquinas de cada cara pueden llevar la
normal propia de esa cara: **flat shading** (sombreado plano) — cada cara un brillo uniforme,
aristas nítidas. Justo lo correcto para el logo; la tetera se verá facetada, lo que es fiel a
su archivo.

**Las normales giran con el modelo.** Cuando el modelo gira, sus caras apuntan a otro sitio,
así que sus normales también deben girar — el mismo giro que la matriz de modelo, sin el
movimiento. Olvídalo y la luz parece pintada: las caras brillantes siguen brillantes gire
como gire el modelo (Q-20.3).

**Dónde deja de valer la analogía:** un sol de verdad proyecta sombras. Esta luz no — una cara
que mira a la luz está iluminada aunque otra parte del modelo se interponga. Las sombras son
otra técnica (ft_vox / más adelante).

---

## Los nombres

**orbit camera** — una cámara sobre una esfera alrededor de un objetivo, mirándolo siempre. Se
controla con yaw, pitch y distancia.

**yaw and pitch** — yaw: rodear el objetivo (izquierda/derecha). Pitch: pasar por encima /
por debajo (arriba/abajo).

**pitch clamp** — parar el pitch un poco antes de la vertical hacia arriba o hacia abajo
(±89°), para que la cámara nunca se dé la vuelta.

**look-at** — la tarjeta de vista ya hecha del engine: "ponte aquí, mira aquello, por aquí es
arriba". scop la usa en vez de la fija `translation(0, 0, −distance)`.

**mouse delta** — cuánto se movió el ratón en este fotograma, en píxeles. La cámara gira según
el *movimiento*, no según dónde esté el cursor.

**directional light** — luz desde una dirección en todas partes (un sol). Solo una dirección,
sin posición.

**normal** — la dirección hacia la que apunta una cara, de longitud 1. Una por esquina,
enviada a los shaders.

**flat shading** — cada esquina de una cara lleva la normal de la cara: un brillo por cara.

**diffuse** — el brillo por mirar a la luz: completo de frente, cero de lado o de espaldas.

**ambient** — un pequeño brillo constante sumado en todas partes, para que las caras de
espaldas no queden negras.

**world-fixed light** — la dirección de la luz pertenece al mundo, no a la cámara: orbitar no
la mueve. (La otra opción, un "foco" fijo a la cámara, siempre ilumina lo que miras.)

**normal transform** — girar las normales con la rotación del modelo (la matriz de modelo sin
su movimiento). scop solo gira y mueve los modelos, nunca los estira, así que no necesita más.

---

## Cómo funciona realmente

### 1. Entrada del ratón (engine)

```
winit CursorMoved / MouseInput → Input::mouse_held(MouseButton::Left)
                                 Input::mouse_delta() → (dx, dy) en píxeles este fotograma
se reinicia en end_frame (como el scroll); el primer movimiento tras pulsar no salta
```

### 2. Cámara orbital (matemáticas del engine + scop)

```
engine: Mat4::look_at(eye, target, up)          // dada + probada, no derivada
scop:   Orbit { yaw, pitch }                    // distancia = la distancia de encuadre de § 12
        arrastrando: yaw −= dx × 0.005, pitch += dy × 0.005 (radianes por píxel)
        pitch limitado a ±89°
        eye = un punto de la esfera alrededor del origen (yaw, pitch, distancia)
        view = look_at(eye, origen, up = +y)    // el modelo ya está centrado en el origen
        R también reinicia la órbita
```

### 3. Normales y luz (vértice + uniform del engine, shaders de scop)

```
engine Vertex: + normal [f32; 3]   (location 3)       // crece el albarán de § 7
engine camera uniform: view_proj + light (vec4)        // crece el tablón de § 10: 64 → 80 bytes
        Frame::set_light(direction, strength)          // strength 0 = iluminación apagada
scop:   normal de esquina = la normal de su cara (face_normals de § 18), con longitud 1
mesh.vert: normal_out = parte de rotación de push.model × normal   // las normales giran con el modelo
mesh.frag: brillo = ambiente + strength × max(0, cuánto mira a la luz)
           modo normal: color × brillo;  modos orientación / profundidad: sin luz (son diagnósticos)
        B activa/desactiva la luz (strength 1 ↔ 0)
```

### Decisiones

| Elección | Frente a | Por qué |
|---|---|---|
| **Cámara orbital al arrastrar con el ratón**, las teclas siguen girando el modelo | Sustituir los controles de teclado | Los controles del subject se quedan; la cámara es un extra |
| **Distancia = distancia de encuadre**, la rueda sigue siendo zoom (§ 18) | Que la rueda acerque/aleje la cámara | El zoom ya existe; W/S ya hacen dolly del modelo |
| **Límite de pitch ±89°** | Rotación libre (cuaterniones) | Nunca se da la vuelta; los cuaterniones son material de ft_newton |
| **`look_at` en el engine** | Construir la vista con rotaciones en scop | Todos los proyectos posteriores lo necesitan |
| **Una luz direccional, fija en el mundo** | Foco en la cámara / luces puntuales | Así orbitar *muestra* el cambio de sombreado (Q-20.1); una luz es el alcance |
| **La luz en el uniform de la cámara** | Más push params | Los push params están llenos (§ 19), y la luz es por fotograma, no por draw (tablón vs pósit de § 10) |
| **Normales planas por esquina** | Normales suaves (promediadas) | Las esquinas ya son por cara (§ 13); correcto para el logo; las suaves son un cambio fácil más adelante |
| **Normales giradas solo con la rotación del modelo** | La inversa traspuesta general | scop nunca estira los modelos; apuntado como trampa para proyectos futuros |
| **Ambiente 0.25** | Sin ambiente | Las caras de espaldas siguen visibles contra el fondo oscuro |

---

## Trampas conocidas

- **La cámara se da la vuelta al pasar por arriba** — pitch sin limitar (Q-20.2).
- **La cámara salta al empezar a arrastrar** — el delta medido desde una posición vieja del
  cursor; el primer movimiento tras pulsar debe empezar desde donde está el cursor ahora.
- **La luz parece pintada** — normales sin girar con el modelo (Q-20.3).
- **Modelo todo negro, o iluminado desde el lado equivocado** — la dirección de la luz apunta
  al revés (hacia la luz vs desde ella), o las normales apuntan hacia dentro. El modo
  orientación (§ 19) comprueba las caras; una luz desde detrás de la cámara debería iluminar
  lo que ves.
- **Error de validación en el vertex input** — `Vertex` creció pero la location 3 del shader o
  la lista de atributos no (§ 7).
- **El uniform lee basura** — el bloque de la cámara creció en el shader pero no en el buffer
  (o al revés): 64 → 80 bytes en los dos lados (§ 10).
- **La tetera se ve facetada** — esperado con flat shading, no es un bug.
- **Modelo estirado mal iluminado** (proyectos futuros) — una escala no uniforme necesita la
  inversa traspuesta para las normales. scop nunca estira, así que aquí no aplica.

---

## Checkpoint

**Cumple (subject):** extras del bonus (cámara orbital, luz direccional). Método: primero el
gate, luego Claude lo escribe y te lo explica.

1. **Engine:** `Input::mouse_held`, `Input::mouse_delta`; `Mat4::look_at` (con tests);
   `Vertex.normal` (location 3); uniform de cámara + vec4 de luz, `Frame::set_light`.
2. **scop:** normales planas a partir de las normales de cara; `Orbit` con arrastre, límite y
   reinicio con R; luz fija en el mundo desde arriba-delante-izquierda; B la activa/desactiva;
   iluminación solo en el modo de color normal; controles y README.
3. **Tests:** `look_at` pone el objetivo justo delante y mantiene arriba como arriba; el límite
   de pitch aguanta; las normales tienen longitud 1 y apuntan hacia fuera en el cubo; el delta
   del ratón se reinicia cada fotograma.

**Verificar:** arrastra alrededor del logo — el lado iluminado entra y sale de la vista
mientras el sombreado de las caras se queda fijo en el modelo; con J/L en su lugar — cambia qué
caras están iluminadas; arrastra con fuerza por encima — no se da la vuelta; B apagar/encender;
modos orientación y profundidad sin luz; validación callada.

---

## Preguntas

Q-20.1 (aplicación) Girar el modelo con J y orbitar la cámara con el ratón se ven igual sin luz.
Con la luz fija en el mundo encendida, ¿qué se ve distinto entre las dos cosas?

Q-20.2 (predicción) El pitch no está limitado y arrastras hacia arriba más allá de la vertical.
¿Qué ves?

Q-20.3 (predicción) Las normales llegan al shader pero no las gira el modelo. Haces girar el
modelo con J / L. ¿Qué hace la iluminación?

Q-20.4 (aplicación) ¿Por qué sumar una cantidad de ambiente en vez de usar solo lo de frente que
mira una cara a la luz?

Q-20.5 (aplicación) ¿Por qué el flat shading depende de que cada triángulo tenga sus propias
esquinas (§ 13)? ¿Qué darían en cambio las esquinas compartidas?

Respuestas: `ANSWERS.md` § 20
