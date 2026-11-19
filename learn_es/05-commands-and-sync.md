---
section: 5
slug: commands-and-sync
tier: T2
status: done
prereqs: [4]
terms: [command pool, recording, vkQueueSubmit, fence, semaphore, acquire, frames in flight, image layout, layout transition, pipeline barrier, pipeline stage, access mask, vkCmdClearColorImage, TRANSFER_DST, device_wait_idle, RedrawRequested]
---

# Sección 5 — Darle trabajo a la GPU, y esperar

**Profundidad: T2** — cada fotograma que scop dibuje pasa por el bucle que construyes aquí,
y casi todo bug del tipo "en mi Mac funciona pero en el campus parpadea / se congela /
revienta" es un bug de sincronización. Tienes que saber explicar *por qué* cada espera está
donde está.
**Serás capaz de:** grabar trabajo para la GPU, enviarlo, mostrar el resultado en pantalla
cada fotograma, y evitar que la CPU, la GPU y la pantalla se pisen entre sí.

---

## La idea

Volvemos al teatro de § 4. Tú — el programa en la CPU — eres ahora el **director**. No
pintas; escribes hojas de instrucciones ("pinta este marco de azul") y se las das a los
pintores (la GPU). Los pintores trabajan por su cuenta, a su ritmo. Mientras pintan, tú
puedes hacer otra cosa — como escribir la siguiente hoja.

Esa libertad es justo lo que se busca, y también el peligro. Hay tres cosas pasando a la
vez, y ninguna espera a las demás salvo que tú lo digas:

1. **Tú, escribiendo hojas.** Si reescribes una hoja mientras los pintores aún la están
   leyendo, pintan disparates. Así que antes de reutilizar una hoja necesitas saber que
   los pintores han terminado con ella. Eso es una **luz en tu mesa** que los pintores
   encienden cuando acaban un trabajo. Puedes pararte a esperar a la luz.
2. **Los pintores.** No deben empezar a pintar un marco que sigue en escena. Así que el
   regidor levanta una **bandera** cuando un marco vuelve libre — y los pintores esperan a
   la bandera antes de empezar. Tú nunca ves esta bandera; es cosa del regidor y los
   pintores.
3. **El regidor.** No debe poner un marco en escena mientras la pintura sigue fresca. Así
   que los pintores levantan una segunda **bandera** cuando terminan, y el regidor la
   espera.

Fíjate en la diferencia entre la luz y las banderas. La **luz** es para *ti* — la CPU se
para y la espera. Las **banderas** son para los pintores y el regidor — se coordinan entre
ellos, y tú solo les dices qué banderas usar.

Un problema más. Antes de pintar sobre un lienzo lo preparas de una manera (imprimado,
tensado); antes de que salga a escena lo preparas de otra (enmarcado, barnizado). El
lienzo es el mismo — solo cambia cómo está dispuesto para cada trabajo. Las GPUs son
igual: la misma imagen se organiza en memoria de forma distinta según vaya a pintarse,
leerse o mostrarse. Tienes que decir cuándo cambiar, y el cambio es en sí mismo una
instrucción de la hoja.

**Dónde deja de valer la analogía:** los pintores no leen las hojas en un orden sensato
solo porque tú las escribieras en orden — la GPU puede solapar pasos si no se le dice lo
contrario. Y "la luz" y "las banderas" son solo pequeños objetos que creas y reutilizas
cada fotograma; nada físico cambia.

---

## Los nombres

**command pool** — `VkCommandPool`: la memoria de la que se recortan los command buffers.
Un pool por queue family (los command buffers solo se pueden enviar a colas de la familia
para la que se hizo su pool). Tú creas el pool; de él *reservas* (allocate) command
buffers. El flag `RESET_COMMAND_BUFFER` te deja reiniciar un buffer cada vez en vez del
pool entero.

**command buffer** — ya está en el glosario desde § 2: la hoja de instrucciones. Lo nuevo
aquí es su ciclo de vida: *allocate* (una vez) → *begin* → grabar comandos → *end* →
*submit* → esperar a que la GPU termine → *reset* → begin otra vez.

**recording** — llamar a funciones `cmd_*` sobre un command buffer entre `begin` y `end`.
Grabar **no hace nada** en la GPU: solo escribe la hoja. Toda función de grabación empieza
por `cmd` — `cmd_clear_color_image`, `cmd_pipeline_barrier`.

**`vkQueueSubmit`** — "**V**ul**k**an: **Queue Submit**" (enviar a la cola). Entrega
command buffers a una cola. Vuelve inmediatamente; la GPU ejecuta el trabajo más tarde.
El submit dice también qué banderas (semáforos) esperar antes de empezar, cuáles levantar
al terminar y qué luz (fence) encender al terminar.

**fence** — `VkFence`: la luz de tu mesa. **La GPU la señala, la CPU espera.**
`wait_for_fences` bloquea la CPU hasta que está encendida; `reset_fences` la vuelve a
apagar. Se llama fence (valla) porque el trabajo a un lado de ella no se puede confundir
con el del otro.

**semaphore** — `VkSemaphore`: una bandera. **La GPU la señala, la GPU (o el motor de
presentación) espera.** La CPU nunca espera estos semáforos. El nombre viene de las
*señales de semáforo* ferroviarias — los brazos en un poste que le dicen a un tren si puede
entrar en el siguiente tramo de vía. (Estrictamente son semáforos *binarios*: arriba o
abajo. Vulkan también tiene semáforos "timeline" que cuentan; scop no los necesita.)

**acquire** — `vkAcquireNextImageKHR`: "dame el índice de una imagen de la swapchain en la
que pueda dibujar a continuación". Vuelve enseguida con un índice — pero puede que la
imagen siga en escena. El semáforo que le pasas se levanta cuando de verdad está libre.

**frames in flight** — cuántos fotogramas puede ir la CPU por delante de la GPU. Con **1**,
escribes una hoja, esperas a que los pintores la terminen y escribes la siguiente: CPU y
GPU se turnan, cada una ociosa la mitad del tiempo. Con **2**, escribes la hoja del
fotograma 2 mientras la GPU pinta el fotograma 1. Cada fotograma en vuelo necesita su
propio command buffer, su fence y su semáforo de "imagen libre". Dos es lo habitual: más
añade retardo de entrada con poca ganancia.

**image layout** — cómo están organizados los píxeles de una imagen en la memoria de la GPU
para un trabajo concreto. `UNDEFINED` (el contenido no importa — puede tirarse),
`TRANSFER_DST_OPTIMAL` (lo mejor para que una copia o un clear *escriba* en ella),
`COLOR_ATTACHMENT_OPTIMAL` (lo mejor para dibujar en ella, desde § 6), `PRESENT_SRC_KHR`
(lista para mostrarse: *present source*). La GPU puede comprimir u organizar en teselas
los píxeles de forma distinta para cada uno.

**layout transition** — cambiar una imagen de un layout a otro. Se graba en el command
buffer y la hace la GPU.

**pipeline barrier** — `vkCmdPipelineBarrier`: un comando grabado que dice "todo lo de
*tipo X* antes de este punto debe terminar antes de que empiece cualquier cosa de *tipo Y*
posterior — y, ya puestos, cambia el layout de esta imagen". Es la instrucción de "no
solapes esto" de la analogía. *Pipeline* porque la GPU funciona como una cadena de
producción con etapas; una *barrier* (barrera) detiene algunas.

**pipeline stage** — un paso de esa cadena de producción: `TRANSFER` (copias y clears),
`COLOR_ATTACHMENT_OUTPUT` (escribir píxeles dibujados, § 6), `TOP_OF_PIPE` /
`BOTTOM_OF_PIPE` ("antes de nada" / "después de todo"). Las barreras y las esperas de
semáforo nombran etapas para que la GPU sepa exactamente *qué* tiene que esperar, y pueda
seguir ejecutando todo lo demás.

**access mask** — *qué tipo de acceso a memoria* debe terminar o hacerse visible:
`TRANSFER_WRITE`, `COLOR_ATTACHMENT_WRITE`, `SHADER_READ`… Una etapa dice *cuándo*, una
access mask dice *qué*. Las GPUs tienen cachés; la access mask es cómo la barrera sabe qué
escrituras hay que volcar para que el siguiente paso las vea.

**`vkCmdClearColorImage`** — **V**ul**k**an **C**o**m**man**d**: limpiar una imagen de
color a un solo color. Lo más sencillo que cambia píxeles, sin shaders ni pipeline. Es una
operación de transferencia, así que la imagen debe estar en `TRANSFER_DST_OPTIMAL`.

**`TRANSFER_DST`** — **transfer destination** (destino de transferencia): un flag de *uso*
de imagen que significa "puede ser el destino de copias y clears". Las imágenes de la
swapchain solo reciben los usos que pidas al crear la swapchain — así que el
`COLOR_ATTACHMENT` de § 4 por sí solo no basta para limpiarlas.

**`device_wait_idle`** — "espera hasta que el dispositivo haya terminado *todo*". A lo bruto
— nunca en el bucle de fotogramas — pero exactamente lo correcto antes de destruir cosas
al salir.

**`RedrawRequested`** — el evento de `winit` que significa "ahora es buen momento para
dibujar esta ventana". Recibes uno cuando lo pide el sistema operativo, o cuando llamas a
`window.request_redraw()`. Llamarlo al final de cada fotograma te da un bucle continuo.

---

## Cómo funciona realmente

### Fotogramas, slots de fotograma e imágenes — tres cosas distintas

Se confunden con facilidad, así que fíjalas primero:

- **Un fotograma** es una vuelta del bucle: una imagen dibujada y mostrada. Ocurre ~60
  veces por segundo, para siempre.
- **Un slot de fotograma** (2 — los "frames in flight") es *tu* kit reutilizable: una hoja
  de instrucciones (command buffer), una luz de mesa (fence), una bandera de "imagen libre"
  (semáforo). Eliges slots **por turnos**: 0, 1, 0, 1… Dos slots significa que la CPU puede
  ir como mucho dos fotogramas por delante de la GPU.
- **Una imagen de la swapchain** (3 en tu Mac) es un *lienzo*: píxeles reales que muestra
  la pantalla. No la eliges tú — **acquire te dice** cuál está libre, en el orden en que el
  motor de presentación las devuelva. Cada imagen tiene su propia view (§ 4) y su propia
  bandera de "render terminado".

| Vuelta | Slot (eliges tú, por turnos) | Imagen (elige acquire) |
|---|---|---|
| 1 | slot 0 | imagen 0 |
| 2 | slot 1 | imagen 1 |
| 3 | slot 0 | imagen 2 |
| 4 | slot 1 | imagen 0 |
| 5 | slot 0 | imagen 1 |

Las dos cuentas son independientes: **3 imágenes** es cuántos lienzos se turnan en
pantalla (lo decidió la swapchain, § 4); **2 slots** es cuánto dejas que la CPU se
adelante (tu elección). Regla práctica de *a qué pertenece un objeto*: si **la CPU lo
reutiliza**, va por slot; si **la pantalla lo retiene**, va por imagen.

### El fotograma entero, de un vistazo

```
CPU (tu código)                           GPU / motor de presentación
───────────────                           ───────────────────────────
wait_for_fences(frame.fence)   ◄────────  fence señalada cuando terminó el fotograma N-2
acquire_next_image(..., frame.image_free) ─► image_free se levanta cuando la imagen sale de escena
reset_fences(frame.fence)
reset + grabar command buffer:
   barrera: UNDEFINED → TRANSFER_DST_OPTIMAL
   clear a un color
   barrera: TRANSFER_DST_OPTIMAL → PRESENT_SRC_KHR
queue_submit:
   esperar  image_free  (en la etapa TRANSFER) ──►  la GPU espera y luego ejecuta los comandos
   señalar render_done[image]                  ──►  se levanta cuando terminan los comandos
   fence frame.fence                           ──►  se enciende cuando terminan los comandos
queue_present:
   esperar render_done[image]                  ──►  el regidor muestra la imagen
frame = (frame + 1) % 2
```

Todo lo de la izquierda vuelve rápido. El único sitio donde la CPU se *para* es
`wait_for_fences` (y, a veces, `acquire`).

### 1. Crear los objetos (una vez)

- **Command pool** para la queue family gráfica, con `RESET_COMMAND_BUFFER`.
- **Por fotograma en vuelo (2):** un command buffer, una fence, un semáforo `image_free`.
- **Por imagen de la swapchain (3 en tu Mac):** un semáforo `render_done`. Por qué por
  *imagen* y no por fotograma: el motor de presentación retiene `render_done` hasta que
  muestra de verdad esa imagen, lo que puede ser después de tu siguiente uso del mismo
  slot. Atarlo a la imagen significa que solo lo reutilizas cuando esa imagen vuelve.
- **Las fences se crean ya señaladas** (`FenceCreateFlags::SIGNALED`). El primer fotograma
  empieza esperando su fence; si la luz empieza apagada, esperas para siempre a un trabajo
  que nunca se envió.

La creación de la swapchain (§ 4) también necesita añadir `TRANSFER_DST` al uso de sus
imágenes — comprueba que los `supported_usage_flags` de la surface lo incluyen.

### 2. Grabar

```
begin_command_buffer(cmd, ONE_TIME_SUBMIT)       // "esta grabación se usa una vez"
pipeline_barrier(
    src stage TRANSFER, src access none,         // esperar a la etapa del semáforo
    dst stage TRANSFER, dst access TRANSFER_WRITE,
    image: UNDEFINED → TRANSFER_DST_OPTIMAL)
clear_color_image(image, TRANSFER_DST_OPTIMAL, colour, whole image)
pipeline_barrier(
    src stage TRANSFER, src access TRANSFER_WRITE, // el clear debe terminar…
    dst stage BOTTOM_OF_PIPE, dst access none,     // …antes de que el fotograma cuente como hecho
    image: TRANSFER_DST_OPTIMAL → PRESENT_SRC_KHR)
end_command_buffer(cmd)
```

`UNDEFINED` como layout antiguo significa "me da igual lo que hubiera" — la GPU puede
ahorrarse conservarlo. Aquí está bien: sobrescribes todos los píxeles. La "imagen entera"
es un *subresource range*: aspecto de color, nivel de mip 0, un nivel, capa 0, una capa —
el mismo rango que las image views de § 4.

### 3. Enviar

El submit espera a `image_free` **en la etapa `TRANSFER`**. Eso es preciso: la GPU puede
empezar ya cualquier trabajo que vaya *antes* de transfer, y solo se contiene en el clear.
Esta es la idea de "la etapa dice cuándo" de la sección de nombres — y la etapa de origen
de la primera barrera es `TRANSFER` para que cuadre con esta espera.

### 4. Presentar

`queue_present` con la swapchain, el índice de imagen y `render_done[image]` para esperar.
Igual que submit, vuelve enseguida.

### 5. Acquire y present pueden decir "la ventana ha cambiado"

Los dos pueden devolver `ERROR_OUT_OF_DATE_KHR` ("la swapchain ya no encaja con la ventana
— reconstrúyela") o informar de *suboptimal* ("sigue funcionando, pero ya no encaja
perfectamente"). Reconstruir es § 16. Por ahora: si está out-of-date, sáltate el
fotograma. **Importante:** si acquire falla, vuelve *antes* de `reset_fences` — si no, la
fence queda apagada sin ningún trabajo que la encienda, y el siguiente fotograma que use
ese slot espera para siempre.

### 6. Moverlo desde `winit`

- En `RedrawRequested`: si la ventana está minimizada (0×0), no hagas nada. Si no, dibuja
  un fotograma y llama a `window.request_redraw()` para pedir el siguiente.
- Con `FIFO`, presentar bloquea más o menos a la frecuencia de refresco de tu pantalla,
  así que este bucle no corre a tope.

### 7. Apagar

Antes de destruir nada: `device_wait_idle()`. Luego destruye en orden espejo — semáforos y
fences, después el command pool (que libera sus command buffers), después los objetos de
§ 4.

### 8. Ownership en Rust

Los objetos de fotograma van en el engine, en su propio struct propiedad de `GpuContext`,
destruidos en su `Drop` *después* de `device_wait_idle` y *antes* de la swapchain. Los
semáforos `render_done` van con las imágenes de la swapchain (uno por imagen), porque si la
swapchain se reconstruye (§ 16) con otro número de imágenes, hay que reconstruirlos
también.

### ¿Y lo de render passes vs dynamic rendering?

Has visto este hilo abierto en `PROGRESS.md`. Limpiar con `vkCmdClearColorImage` no
necesita ninguno de los dos, así que la decisión pasa a § 6, donde el pipeline gráfico sí
necesita uno — allí se expondrán los pros y contras. MoltenVK en tu Mac reporta **Vulkan
1.2**, que es el dato principal.

---

## Trampas conocidas

(Los números `VUID-…` de abajo son los que imprimen las validation layers recientes; una
más antigua puede redactarlos de forma algo distinta.)

- **Se cuelga en el primerísimo fotograma** — la fence se creó sin señalar. Créala con
  `SIGNALED`.
- **Se cuelga tras redimensionar** — reiniciaste la fence y luego volviste antes porque
  acquire dijo out-of-date. Reinicia solo después de que acquire tenga éxito.
- **`VUID-vkBeginCommandBuffer-commandBuffer-00049`** ("command buffer is in use") —
  volviste a grabar un command buffer que la GPU todavía está ejecutando. Te saltaste o
  colocaste mal la espera de la fence.
- **`VUID-vkQueueSubmit-pSignalSemaphores-00067`** o un aviso sobre un semáforo aún en uso
  por present — `render_done` era por fotograma en vez de por imagen de la swapchain.
- **`VUID-VkPresentInfoKHR-pImageIndices-01430`** ("image layout … expected
  `VK_IMAGE_LAYOUT_PRESENT_SRC_KHR`") — olvidaste la transición de layout final.
- **Error de validación de que la imagen no se creó con `TRANSFER_DST`** — no añadiste el
  flag de uso a la swapchain.
- **Errores al salir sobre objetos "in use"** — destruiste cosas mientras la GPU seguía
  ocupada. Llama antes a `device_wait_idle`.
- **"Limpié a gris 0.5 y se ve demasiado claro"** — el formato de la swapchain es `_SRGB`
  (§ 4). Tu 0.5 significa "la mitad de luz", que la codificación sRGB guarda como
  aproximadamente 0.73. Comportamiento correcto, no un bug.
- **Solo se dibuja un fotograma** — no llamaste a `request_redraw()` tras dibujar. Un
  color fijo lo oculta; un color que cambia con el tiempo lo delata.

---

## Checkpoint

**Cumple:** "dentro de una ventana" — la ventana ya muestra píxeles que pusiste tú, cada
fotograma. (Método: primero el gate, luego Claude lo escribe y te lo explica.)

1. Las imágenes de la swapchain reciben el uso `TRANSFER_DST` (comprobado contra la
   surface).
2. Un struct del engine para los objetos por fotograma: command pool, 2 command buffers, 2
   fences (señaladas), 2 semáforos `image_free`; semáforos `render_done`, uno por imagen de
   la swapchain.
3. `draw_frame(clear_colour)` haciendo exactamente el bucle de arriba, incluido el manejo
   de out-of-date.
4. `winit` lo mueve desde `RedrawRequested`, pide el siguiente redibujado y se lo salta
   mientras está minimizada.
5. El color pulsa lentamente con el tiempo — prueba de que se dibuja cada fotograma.
6. `device_wait_idle` antes de desmontar; destrucción limpia.
7. **Demuestra que la fence importa:** crea las fences sin `SIGNALED`, ejecuta, míralo
   colgarse, vuelve a ponerlo.

**Verificar:** una ventana que pulsa lentamente; ningún mensaje de validación mientras
corre ni al salir; `cargo clippy` limpio. Redimensionar puede parar el pulso
(out-of-date) — esperado hasta § 16.

---

## Preguntas

Q-5.1 (recuerdo) Para una fence y para un semáforo: ¿quién la/lo señala y quién la/lo
espera?

Q-5.2 (predicción) Creas las dos fences de fotograma sin el flag `SIGNALED`. ¿Qué pasa al
ejecutar scop, y por qué?

Q-5.3 (aplicación) Ordena estos pasos para un fotograma, y di qué semáforo espera o levanta
cada paso de submit/present: *submit*, *present*, *esperar fence*, *acquire*, *grabar*,
*reiniciar fence*.

Q-5.4 (predicción) Borras la segunda barrera (la que pasa la imagen a `PRESENT_SRC_KHR`).
¿Qué reporta la validación, y qué podrías ver en un driver que no se queje?

Q-5.5 (predicción) Cambias a 1 fotograma en vuelo: un command buffer, una fence. El
programa sigue funcionando. ¿Qué hace la CPU mientras la GPU dibuja, y qué le permite hacer
en su lugar el segundo juego de objetos?

Respuestas: `ANSWERS.md` § 5
