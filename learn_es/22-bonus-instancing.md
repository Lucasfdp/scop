---
section: 22
slug: bonus-instancing
tier: T1
status: done
prereqs: [10, 15, 19, 20]
terms: [draw call, instancing, instance, instance index, first instance, instance buffer, storage buffer, draw-call overhead]
---

# Sección 22 — Cientos de copias en un solo draw (bonus)

**Profundidad: T1, matemáticas mínimas (elección del propietario, 2026-11-19).** Planificada
como T2. Esta sección dice qué es el instancing, dónde vive cada pieza y cómo verlo funcionar;
las matrices de cada copia se construyen con las tarjetas que ya tienes (§ 9), no con
matemáticas nuevas.
Extra del bonus (elegido por el propietario, 2026-11-18): **cientos de logos girando con una
mesh y un draw**.
**Serás capaz de:** pulsar G y llenar la pantalla con 512 copias del modelo, cada una girando
a su manera — dibujadas con **un** comando de draw por objeto, no 512.

---

## La idea

**Lo que cuesta un draw.** Cada `draw_textured` hasta ahora se convierte en unos pocos comandos
en el command buffer del fotograma (§ 5): enlazar el pipeline, enlazar la mesh, empujar el
pósit, *dibujar*. Grabarlos es trabajo de la CPU, y la GPU tiene un pequeño coste al empezar
cada draw. Un draw por fotograma no es nada. 512 draws del mismo logo — la misma mesh, la misma
imagen, solo un sitio distinto para cada uno — repiten los mismos comandos 512 veces sin ningún
buen motivo.

El **instancing** lo dice una vez: "dibuja esta mesh **512 veces**". La GPU ejecuta el vertex
shader para cada esquina de cada copia, y le dice al shader **qué copia** está procesando: el
**índice de instancia**, de 0 a 511. El shader lo usa para buscar la matriz propia de esa copia
en una lista. Misma mesh, misma imagen, mismo pipeline — un comando.

**Dónde vive la lista.** 512 matrices no caben en el pósit (el bloque de push cabe más o menos
una). Van en un **buffer** que el vertex shader puede leer como un array: un **storage
buffer**. La CPU lo reescribe cada fotograma (las copias giran), así que — exactamente como la
cámara (§ 10) — hay uno por slot de fotograma, o la CPU sobrescribiría una lista que la GPU aún
está leyendo.

**Un shader para todo.** El vertex shader de scop ahora siempre hace: *modelo del pósit ×
matriz de esta copia*. Un draw normal es solo "1 copia, cuya matriz no cambia nada": la lista
siempre empieza con esa matriz que no hace nada (la identidad, § 9) en la posición 0, y los
draws normales leen la posición 0. Así sigue habiendo un solo pipeline por estilo, y todo lo
anterior (imágenes, fundido, colores de depuración, luz, recarga en caliente) también funciona
con la multitud.

**La multitud.** G alterna entre el modelo y una multitud: una cuadrícula de 8 × 8 × 8 copias,
reducidas para que toda la multitud quepa donde estaba el modelo, cada una girando a su propia
velocidad. Tus controles (flechas, W/S, I–O, la órbita) siguen moviendo y girando la multitud
entera.

**Dónde deja de valer la analogía:** "un draw" no significa "512 veces más barato". La GPU
sigue dibujando cada triángulo de cada copia: 512 teteras son 3 millones de triángulos. El
instancing ahorra la *repetición* — la grabación en la CPU y el arranque de cada draw —, no el
dibujo en sí.

---

## Los nombres

**draw call** — un comando "dibuja esta mesh" grabado en el command buffer
(`cmd_draw_indexed`).

**instancing** — dibujar una mesh muchas veces con una sola draw call.

**instance** — una copia en un draw con instancing.

**instance index** — qué copia está procesando el vertex shader (`gl_InstanceIndex`), contando
desde la *first instance* del draw.

**first instance** — dónde empiezan en la lista las copias de un draw. Draws normales: 0 (la
identidad). La multitud: de 1 en adelante.

**instance buffer** — la lista de datos por copia, aquí una matriz por copia, reescrita cada
fotograma, una por slot de fotograma.

**storage buffer** — un buffer que un shader lee como un array de cualquier longitud
(`readonly buffer` en GLSL). Un uniform buffer (§ 10) es para bloques pequeños de tamaño fijo.

**draw-call overhead** — el coste por draw en la CPU (grabar) y en la GPU (arrancar un draw),
que se vuelve a pagar en cada draw, por pequeña que sea la mesh.

---

## Cómo funciona realmente

### 1. Engine: la lista de instancias y el draw con instancing

```
set 0 (por slot de fotograma): binding 0 = cámara (§ 10)   binding 1 = lista de instancias (storage buffer)
    capacidad 4096 matrices; la posición 0 siempre es la identidad
Frame::draw_instances(pipeline, mesh, texture, model, params, &[Mat4])   // las matrices de las copias
    draws normales:       first instance 0, count 1           (sin cambios para ninguna app)
    draws con instancing: matrices empaquetadas tras la identidad, first instance = donde empiezan
grabación: escribir la lista del slot (tras su fence, como la cámara) → cmd_draw_indexed(count, first)
demasiadas copias para la lista → dibujar las que caben, avisar una vez (nunca desbordar el buffer)
```

### 2. scop: shader y la multitud

```
mesh.vert: layout(set = 0, binding = 1) readonly buffer Instances { mat4 instance[]; };
           model = push.model × instance[gl_InstanceIndex]
           (las normales giran con el mismo modelo combinado — § 20 — así la luz está bien en cada copia)
scop:      G alterna la multitud;  multitud = 8 × 8 × 8 = 512 copias en cuadrícula, reducidas para caber en el espacio del modelo
           copia i: sitio en la cuadrícula, su propio giro (velocidad y eje variados según i), su propio tamaño pequeño
           matrices reconstruidas cada fotograma con las tarjetas que conoces: mover × girar × tamaño (§ 9)
           multitud activada → un draw_instances por objeto; desactivada → draw_with como antes
```

### Decisiones

| Elección | Frente a | Por qué |
|---|---|---|
| **Storage buffer + `gl_InstanceIndex`** | Un segundo vertex binding con ritmo `INSTANCE` (la forma de los libros) | La disposición del vértice (el contrato del shader, § 21) se queda como está; se pueden añadir datos por copia más adelante |
| **Identidad en la posición 0, leída por los draws normales** | Shaders / pipelines separados con y sin instancing | Un pipeline por estilo; todo lo anterior funciona con la multitud |
| **Una lista por slot de fotograma** | Una lista compartida | La CPU la reescribe cada fotograma — si no, la carrera de § 10 |
| **Capacidad fija (4096), dibujar lo que cabe + avisar** | Hacer crecer el buffer bajo demanda | Sencillo; 4096 sobra para scop. Hacerlo crecer es para ft_vox |
| **Matrices construidas en la CPU cada fotograma** | Calcular el giro en el shader a partir del índice de instancia y el tiempo | Reutiliza las tarjetas de § 9; el shader se queda general |
| **8 × 8 × 8 = 512 copias, G alterna** | Un número que puedas cambiar | "Cientos" con una tecla; fácil de cambiar en una constante |

---

## Trampas conocidas

- **El modelo único se encoge o se mueve con la multitud desactivada** — las matrices de la
  multitud se escribieron en la posición 0, encima de la identidad que leen los draws normales
  (Q-22.3).
- **Las copias parpadean o saltan** — una lista compartida para todos los slots de fotograma: la
  CPU la reescribe mientras la GPU la lee (§ 10, Q-22.2).
- **Error de validación al crear el pipeline** — el vertex shader lee el binding 1 del set 0,
  pero el layout del set no lo tiene (o no para la etapa de vértices). El shader y el layout
  cambian juntos.
- **Luz incorrecta en las copias** — normales giradas solo por `push.model`, no por la matriz
  de la copia.
- **Los FPS siguen diciendo 60** — el techo de vsync (§ 19). Esconde cuánto más hace la GPU
  (Q-22.4).
- **Todas las copias idénticas** — no se usa el índice de instancia (todas leen la posición 0),
  o `first instance` se dejó en 0 para la multitud.

---

## Checkpoint

**Cumple (subject):** extra del bonus (instancing). Método: primero el gate, luego Claude lo
escribe y te lo explica.

1. **Engine:** storage buffer en el binding 1 del set 0 por slot (identidad en 0, capacidad,
   aviso una vez); `Frame::draw_instances`; `cmd_draw_indexed` con número de instancias y first
   instance.
2. **scop:** `mesh.vert` lee la lista de instancias; G alterna una multitud de 512 copias;
   matrices por copia cada fotograma; controles, README.
3. **Tests:** empaquetado (draws normales en 0, los de instancing después, en orden); por encima
   de la capacidad dibuja lo que cabe; la multitud cabe dentro de la esfera del modelo; las copias
   no se solapan; 512 matrices, todas distintas.

**Verificar:** G sobre el logo — 512 logos, cada uno girando a su manera; órbita, W/S, B (luz por
copia), C (orientación/profundidad por copia), N, V funcionan con la multitud; G desactivado — el
modelo único exactamente como antes; la multitud de teteras sigue funcionando; validación
callada.

---

## Preguntas

Q-22.1 (recuerdo) Dibujando el logo 512 veces: ¿qué ahorra un draw con instancing frente a 512
draws separados — y qué *no* ahorra?

Q-22.2 (aplicación) ¿Por qué las 512 matrices van en un buffer y no en el bloque de push, y por
qué un buffer por slot de fotograma en vez de uno compartido?

Q-22.3 (predicción) Un bug escribe las matrices de la multitud empezando en la posición 0 de la
lista. Pulsas G para desactivar la multitud. ¿Qué aspecto tiene el modelo único?

Q-22.4 (predicción) Con G activado, 512 teteras, el título sigue mostrando 60 fps. ¿Significa eso
que la multitud no cuesta nada? ¿Qué está escondiendo el número?

Q-22.5 (aplicación) Cada copia gira a su manera, y sin embargo el código de la luz (§ 20) no se
cambió. ¿Por qué la luz sigue estando bien en cada copia?

Respuestas: `ANSWERS.md` § 22
