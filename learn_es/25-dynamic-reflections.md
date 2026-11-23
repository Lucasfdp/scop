---
section: 25
slug: dynamic-reflections
tier: T1
status: done
prereqs: [5, 11, 23, 24]
terms: [render to texture, render target, reflection probe, cube face camera, probe exclusion, offscreen pass, layout hand-over]
---

# Sección 25 — Reflejos en vivo (bonus)

**Profundidad: T1, sin matemáticas (como §§ 20–24).** Petición del propietario (2026-11-21): el
cromado debe reflejar **lo que de verdad tenga alrededor** — el color de fondo, el cielo si está
activado, cualquier otra cosa de la escena —, no una imagen elegida de antemano. Construido
directamente en el engine (regla de § 24).
**Serás capaz de:** explicar cómo un renderizador dibuja en una textura en vez de en la pantalla,
por qué un reflejo en vivo son seis vistas extra de la escena, y cuánto cuesta.
**También en este checkpoint (petición del propietario, sin sección de aprendizaje):** los
controles escritos en pantalla, que se muestran u ocultan con H.

---

## La idea

**Dibujar en una imagen.** Hasta ahora cada fotograma se dibujaba en una imagen de la swapchain —
la pantalla. Nada impide que la GPU dibuje en una imagen normal: un **render target**. Mismos
pipelines, mismos draws, mismo test de profundidad; simplemente el resultado no se muestra. Es una
imagen que has hecho tú, y un shader puede leerla después como cualquier textura (§ 14). Esto es
**render to texture** (renderizar a textura), y es la puerta a la mayoría de efectos "avanzados":
reflejos, sombras, espejos, pantallas de cámaras de seguridad, postprocesado.

**Un cube map en vivo.** El cube map de § 23 eran seis imágenes hechas una vez al arrancar. Uno en
vivo son seis imágenes **hechas cada fotograma**: pon una cámara en el centro del modelo, apúntala a
cada una de las seis paredes por turnos (derecha, izquierda, arriba, abajo, delante, detrás) y dibuja
la escena cada vez con una vista cuadrada de 90° — seis vistas que juntas ven en todas las
direcciones. Ese punto es una **sonda de reflejos** (reflection probe). El cromado lee entonces este
cubo en vez del cielo fijo: fondo verde → reflejo verde; cielo activado → cielo; cualquier otra cosa
de la escena → en el reflejo (Q-25.3).

**Deja el espejo fuera de su propia foto.** La sonda está *dentro* del modelo. Dibujar el modelo en
su propio cubo llenaría cada cara con el interior del modelo, y el cromado reflejaría eso. Así que los
draws reflectantes se **excluyen** de las vistas de la sonda (Q-25.1) — el mismo límite honesto que en
§ 23: el modelo nunca se refleja a sí mismo.

**El orden importa.** Las seis vistas de la sonda tienen que estar terminadas antes de que el cromado
las lea, en el mismo fotograma. Entre una cosa y otra, la imagen cambia de trabajo — de "se está
dibujando en ella" a "la leen los shaders" — con una barrera, como el traspaso de la imagen de la
swapchain en § 5 (Q-25.4).

**Cuesta.** Seis vistas extra de la escena, cada fotograma: hasta siete veces el dibujo. Así que el
engine solo lo hace cuando se lo piden (scop lo pide solo en modo cromado), y las caras de la sonda
son pequeñas — 256 × 256 — porque un reflejo es una parte pequeña, curva y borrosa de la pantalla
(Q-25.5).

**Dónde deja de valer la analogía:** un espejo de verdad muestra el mundo desde donde está *cada
punto* del espejo. Una sonda muestra el mundo desde **un** punto, el centro del modelo. En un modelo
grande o alargado, los reflejos cerca de sus extremos quedan ligeramente fuera de sitio. Los juegos lo
aceptan; scop también.

---

## Los nombres

**render to texture** — dibujar en una imagen normal en vez de en la pantalla, y luego usar el
resultado como textura.

**render target** — la imagen en la que dibuja una pasada (color, más su propia imagen de
profundidad).

**reflection probe** — un punto donde se captura la escena en un cube map para los reflejos.

**cube face camera** — una de las seis cámaras de una sonda: vista cuadrada, 90° de ancho, mirando a
una pared del cubo. Las seis juntas ven cada dirección exactamente una vez.

**probe exclusion** — draws que se dejan fuera de las vistas de la sonda (el propio modelo
reflectante).

**offscreen pass** — una ronda de dibujo que no se muestra (aquí: las seis vistas de la sonda),
grabada antes de la pasada principal en el mismo command buffer.

**layout hand-over** — la barrera que pasa la imagen de la sonda de "se dibuja en ella" a "la leen los
shaders" (y de vuelta el fotograma siguiente), como las transiciones de la swapchain de § 5.

---

## Cómo funciona realmente

### 1. Engine: la sonda

```
ReflectionProbe (una, propiedad del engine): imagen de cubo 256², 6 capas, usable como
    render target Y como textura muestreada; una view 2D por capa (para dibujar en ella),
    una view de cubo (para leer); su propia imagen de profundidad
Frame::set_reflection_probe(position)          // este fotograma, capturar en `position`
las draw calls reciben un flag: .in_reflections (true por defecto); la app lo quita
    para el modelo reflectante (p. ej. Frame::draw_with + exclude_from_reflections())
grabación, antes de la pasada principal:
    barrera: cubo → "se dibuja en él"
    para cada cara: cámara = cámara de cara en `position` → dibujar cada call incluida
    barrera: cubo → "lo leen los shaders"
el entorno de este fotograma = el cubo de la sonda (set 0, binding 2);
    sin sonda este fotograma → el entorno fijo (§ 23), sin cambios
siete cámaras por fotograma (principal + 6 caras): el buffer de cámara guarda las siete
```

### 2. scop

```
modo cromado → frame.set_reflection_probe(centro del modelo en el mundo); los draws del
    modelo se excluyen; M alterna en vivo ↔ fijo (fijo = el cielo de § 23)
todo lo demás (cielo, fondo) aparece automáticamente en las vistas de la sonda
```

### 3. Extra (petición del propietario, sin sección de aprendizaje): controles en pantalla

```
engine: una fuente de mapa de bits incorporada (de ancho fijo, glifos de dominio público, una
    imagen creada al arrancar), Frame::draw_text(text, position, scale), dibujada la última en
    píxeles de pantalla con un pipeline de superposición (sin test de profundidad, con mezcla),
    reutilizable por cualquier proyecto
scop: la lista de controles en una esquina; H la muestra / oculta (visible al arrancar)
```

### Decisiones

| Elección | Frente a | Por qué |
|---|---|---|
| **Una sonda en el centro del modelo** | Una por objeto | Una captura por fotograma; basta para "reflejar lo que hay alrededor"; las sondas por objeto multiplican el coste |
| **Excluir los draws reflectantes** | Dibujarlo todo | La sonda está dentro del modelo: su propio interior llenaría el reflejo |
| **Caras de 256², solo cuando se pide** | Del tamaño de la ventana, cada fotograma | Los reflejos son pequeños en pantalla; nadie paga cuando el cromado está apagado |
| **Mismos draws, mismos pipelines** | Shaders especiales para la sonda | Lo que muestra la vista principal lo muestra la sonda — de eso se trata |
| **M: en vivo ↔ fijo** | Solo en vivo | Mantiene el aspecto de § 23 a una tecla, y una forma de comparar |
| **Texto superpuesto con una fuente de mapa de bits incorporada** | Una librería de fuentes / TrueType | Las librerías solo se permiten para ventanas y eventos; una fuente de mapa de bits es una imagen pequeña |

---

## Trampas conocidas

- **Reflejo lleno de caras oscuras del revés** — no se excluyó el modelo reflectante (Q-25.1).
- **Reflejo con un fotograma de retraso, parpadeando, o un error de validación sobre layouts** — el
  cromado lee el cubo antes de que terminen las seis vistas, o sin la barrera de traspaso (Q-25.4).
- **Cielo boca abajo o de lado en el reflejo en vivo** — el "arriba" de una cámara de cara no coincide
  con la tabla de caras del cubo (§ 23). Comprobación: con el cielo activado, el cromado en vivo y el
  fijo deberían verse igual (M).
- **La tasa de fotogramas se desploma con la multitud** — la sonda dibuja la escena seis veces más; la
  multitud excluida se salta, pero todo lo incluido se paga siete veces (Q-25.5).
- **Todas las caras muestran la misma vista** — las seis cámaras de cara recibieron el mismo buffer de
  cámara: siete cámaras por fotograma, una por vista.
- **Texto borroso o corrido** — la imagen de la fuente se muestrea mezclando entre texels; las letras
  quieren píxeles nítidos (muestreo nearest).

---

## Checkpoint

**Cumple (subject):** extra del bonus (reflejos en vivo) + controles en pantalla. Método: primero el
gate, luego Claude lo escribe y te lo explica.

1. **Engine:** `ReflectionProbe` (render target de cubo + profundidad, views por capa y de cubo);
   `Frame::set_reflection_probe`; exclusión por draw; seis pasadas offscreen antes de la pasada
   principal con las barreras de traspaso; siete cámaras por fotograma; el cubo de la sonda como
   entorno de este fotograma.
2. **scop:** el cromado pone la sonda en el modelo y excluye el modelo; M en vivo ↔ fijo.
3. **Extra (sin sección de aprendizaje):** fuente de mapa de bits en el engine + `Frame::draw_text` +
   pipeline de superposición; scop muestra los controles, H los alterna.
4. **Tests:** las seis cámaras de cara miran a lo largo de +x, −x, +y, −y, +z, −z y juntas cubren cada
   dirección una vez; los draws excluidos nunca llegan a la lista de la sonda; sin sonda → el entorno
   fijo; disposición del texto (saltos de línea, posiciones de los caracteres); cada carácter
   imprimible tiene un glifo.

**Verificar:** cielo activado, cromado, M — en vivo y fijo se ven igual; cielo desactivado — el cromado
en vivo refleja el fondo oscuro (cambia `BACKGROUND` a verde para verlo claro), el fijo sigue mostrando
el cielo; FPS con cromado en vivo frente a normal; H oculta / muestra los controles; validación callada.

---

## Preguntas

Q-25.1 (aplicación) ¿Por qué se dejan fuera los propios draws del modelo reflectante cuando la sonda
captura su cubo?

Q-25.2 (aplicación) ¿Por qué se vuelve a capturar el cubo cada fotograma, en vez de una vez al arrancar
como el de § 23?

Q-25.3 (predicción) Cielo desactivado, `BACKGROUND` cambiado a verde. En modo cromado, ¿qué muestra el
modelo con reflejos **fijos** (§ 23), y qué con reflejos **en vivo**?

Q-25.4 (predicción) Se dibujan las seis vistas de la sonda, pero se olvida la barrera de traspaso antes
de que el cromado lea el cubo. ¿Qué podrías ver, y diría algo la validación?

Q-25.5 (aplicación) ¿Por qué el engine captura la sonda solo cuando la app lo pide (modo cromado), y a
256 × 256 por cara en vez de al tamaño de la ventana?

Respuestas: `ANSWERS.md` § 25
