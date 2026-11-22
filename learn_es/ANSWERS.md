# Respuestas — scop

Léelas después de intentarlo. Las respuestas explican el razonamiento, no solo el dato.

## Sección 1

**A-1.1** `&T` es un préstamo compartido (solo lectura); `&mut T` es un préstamo mutable
(lectura-escritura). En cada momento un valor puede tener **o bien** cualquier número de `&T`
**o bien** exactamente un `&mut T` — nunca ambos. El motivo: si algo puede escribir mientras
otros leen, los lectores pueden ver datos a medio actualizar, o datos que se han movido o
liberado.

**A-1.2** **No** compila (`E0502`). `first` es un préstamo compartido dentro del vector, y
`push` necesita un préstamo mutable mientras `first` sigue usándose en la última línea. La
regla previene un **use-after-free**: `push` puede realocar el buffer del vector a otra
dirección, dejando `first` apuntando a memoria liberada — exactamente lo que pasa en C con
un puntero dentro de un array al que se le hace `realloc`. Arreglo: termina de usar `first`
antes del push, o copia/clona el valor antes.

**A-1.3** Se destruye primero `instance`, porque los campos de un struct se destruyen en
**orden de declaración**. Vulkan exige lo contrario: todo objeto creado a partir de la
instancia (el dispositivo incluido) debe destruirse *antes* que la instancia. Destruir la
instancia primero deja al dispositivo refiriéndose a un padre muerto — la validation layer
lo avisa y en algunos drivers revienta. Arreglo: declarar los campos en orden inverso al de
creación o (más claro, y lo que hace § 3) implementar `Drop` en el struct padre y destruirlo
todo explícitamente en el orden correcto.

**A-1.4** `?` devuelve el error *tal cual* desde tu función, así que los tipos de error deben
coincidir — o el error debe poder convertirse con `From`. El tipo de error de tu función es
`String`, pero el error es `std::io::Error`, y no hay conversión automática.
Arreglos (cualquiera):
- convertir a mano: `fs::read_to_string(path).map_err(|e| e.to_string())?`
- o crear tu propio enum de error con `impl From<std::io::Error> for MyError`, para que `?`
  convierta solo — la mejor opción para el crate del motor, porque quien llame puede hacer
  `match` sobre *qué* error ocurrió.

**A-1.5** `CString::new("scop").unwrap()` crea un `CString` **temporal** que se destruye al
final de la sentencia. `.as_ptr()` presta dentro de él como puntero *crudo*, que el borrow
checker no rastrea — así que compila, pero `p` cuelga desde la línea siguiente. Pasarlo a
Vulkan lee memoria liberada: nombres de aplicación basura, o un crash. Arreglo:
`let name = CString::new("scop")?;` en su propia línea, luego `name.as_ptr()`, y mantener
`name` vivo hasta que Vulkan haya terminado con él.

## Sección 2

**A-2.1** El **vertex shader** se ejecuta una vez por vértice; el **fragment shader** una vez
por fragmento (cada píxel que cubre un triángulo, antes del test de profundidad). Todo lo
demás — primitive assembly, rasterización, depth test — es función fija: lo configuras, no
lo programas.

**A-2.2** Un cubo tiene 6 caras; cada cara cuadrada se divide en 2 triángulos, así que 12
triángulos × 3 vértices = **36**. Las esquinas las comparten varias caras, pero se envían una
vez por cada triángulo que las usa. (En § 7 un **index buffer** te deja enviar las 8 esquinas
únicas una sola vez y 36 índices pequeños — aunque para colores por cara a menudo
necesitarás vértices separados por cara de todas formas, porque una esquina compartida por
tres caras necesitaría tres colores distintos.)

**A-2.3** Un **degradado** suave: rojo puro en una esquina, verde y azul en las otras,
mezclándose por dentro. Lo hace la **rasterización**: interpola cada salida de los vértices
a lo largo del triángulo, ponderada por la distancia del píxel a cada esquina, antes de que
lo vea el fragment shader. Por eso los colores planos por cara requieren trabajo extra (§ 13)
— por defecto, todo se mezcla.

**A-2.4** Podrías ver **los datos viejos, los nuevos o una mezcla** — parpadeos, triángulos
estirados o un fotograma corrupto — y puede cambiar de una ejecución a otra. El envío vuelve
al instante; la GPU lee los datos de vértices *después*, cuando tu código Rust ya ha seguido.
Has cambiado memoria que la GPU aún está leyendo. Es una **carrera entre CPU y GPU**; el
arreglo es la sincronización (fences, § 5) y tener una copia de los datos por fotograma para
cada fotograma en vuelo.

**A-2.5** La persona aparece **boca abajo**. En NDC de Vulkan +y apunta *hacia abajo* en la
pantalla; en OpenGL, *hacia arriba*. Un shader escrito para OpenGL pone "arriba" abajo. (La
profundidad también cambia: Vulkan espera 0..1 y OpenGL −1..1, lo que puede recortar o
distorsionar la profundidad.) Ambas cosas se arreglan una vez en tu matriz de proyección
(§ 10), no dando la vuelta al modelo.

## Sección 3

**A-3.1** Creación: **instancia → (debug messenger) → dispositivo lógico** — el physical
device se *elige*, no se crea, y la cola se *obtiene* del dispositivo. La destrucción es el
espejo: **dispositivo → debug messenger → instancia**. Hijos antes que padres, siempre: todo
lo creado a partir del dispositivo debe irse antes que el dispositivo, y el dispositivo y el
messenger se crean a partir de la instancia.

**A-3.2** MoltenVK es un driver de *portabilidad*, y el loader oculta los drivers de
portabilidad a menos que lo pidas expresamente. Los tutoriales de Windows nunca lo
necesitan. Arreglo: (1) activar la extensión de instancia `VK_KHR_portability_enumeration`,
y (2) poner el flag `ENUMERATE_PORTABILITY_KHR` en el create info de la instancia. Después,
en el dispositivo, activar `VK_KHR_portability_subset` porque el dispositivo la lista.

**A-3.3** **Familia 1** — la única con `GRAPHICS`, y dibujar necesita una cola gráfica. Las
familias 0 y 2 no pueden dibujar. (Los engines reales a veces usan además una familia solo
de transferencia como la 2 para subidas rápidas, pero eso es una optimización que scop no
necesita.)

**A-3.4** **Nada.** En un build release `cfg!(debug_assertions)` es false, así que tu código
nunca activa la validation layer — no hay inspector, así que nadie se entera de la fuga. El
programa sale con normalidad y el sistema operativo limpia el buffer junto con el proceso.
Por eso pruebas con builds de depuración, y por eso "funciona sin errores" en release no
demuestra nada sobre que sea correcto.

**A-3.5** El primer `destroy_device` lo destruye; el segundo destruye un objeto ya destruido
— **comportamiento indefinido**: un crash, un error de validación (si la layer está activa)
o corrupción silenciosa. Es un double free, edición Vulkan: un handle es solo un número, así
que copiarlo no copia el dispositivo. Rust ayuda si el engine nunca reparte handles
`VkDevice` en crudo para que alguien los posea: un struct posee el dispositivo y lo destruye
en su `Drop`, y ese struct **no es `Copy` ni `Clone`** — así que solo puede haber un dueño, y
un destroy.

## Sección 4

**A-4.1** La surface es el adaptador entre Vulkan y la ventana de una plataforma: "un sitio
al que puedo presentar imágenes". El núcleo de Vulkan no sabe de ventanas porque tiene que
funcionar en todas partes — servidores sin pantalla, móviles, consolas, todos los sistemas
de ventanas de escritorio — y cada uno tiene una idea distinta de lo que es una ventana. Así
que las ventanas son una extensión (`VK_KHR_surface` más una por plataforma, p. ej.
`VK_EXT_metal_surface`).

**A-4.2** **3** — `min_image_count + 1`. `max_image_count = 0` es un valor especial que
significa *sin límite superior*, no "se permiten cero imágenes", así que no hay nada a lo que
limitar. El `+1` da una imagen de repuesto para que la CPU/GPU pueda empezar el siguiente
fotograma sin esperar a que el motor de presentación suelte una.

**A-4.3** Tu ventana lógica es de 1280×720 pero la pantalla Retina tiene el doble de píxeles
físicos: 2560×1440. Pedir 1280×720 o bien provoca un error de validación (la surface reporta
un extent actual fijo con el que debes coincidir) o bien da una imagen de un cuarto del
tamaño, estirada — borrosa, o dibujada en una esquina. Usa el tamaño **físico**:
`current_extent` de las surface capabilities, o `window.inner_size()` cuando el extent
actual es el valor especial de "elige tú".

**A-4.4** Minimizar deja la ventana en 0×0. Una swapchain con extent cero no es válida: la
creación falla o la validation layer da un error, y algunos drivers revientan. Arreglo:
cuando el tamaño es cero, sáltate el renderizado y la reconstrucción hasta que la ventana
vuelva a tener un tamaño de verdad.

**A-4.5** Porque la instancia y la elección de dispositivo ahora dependen de la ventana: (1)
**crear la instancia** necesita las extensiones del sistema de ventanas, que se obtienen del
display handle de la ventana, y (2) **elegir el physical device / queue family** necesita la
surface, que se crea a partir de la ventana — tienes que comprobar que la familia puede
presentar en ella. Como `winit` solo te da la ventana en `resumed`, la configuración de
Vulkan se muda ahí.

## Sección 5

**A-5.1** **Fence:** la GPU la señala cuando termina un lote enviado; la **CPU** la espera
(`wait_for_fences`). Es como sabe tu código que puede reutilizar un command buffer.
**Semáforo:** lo señala la GPU (o `acquire`, cuando la imagen sale de pantalla); lo espera
la **GPU** (un submit) o el **motor de presentación** (un present). La CPU nunca espera un
semáforo binario — solo nombra cuáles usar.

**A-5.2** **Se cuelga en el primer fotograma.** Lo primero que hace el fotograma es
`wait_for_fences` sobre la fence de su slot — pero nunca se ha enviado nada con esa fence,
así que nada la va a señalar nunca. La CPU espera para siempre: la ventana se abre pero se
queda en blanco y deja de responder. Crear las fences `SIGNALED` significa "el uso anterior
(imaginario) de este slot ya terminó", así que la primera espera pasa enseguida.

**A-5.3** **esperar fence → acquire → reiniciar fence → grabar → submit → present.**
- a *acquire* se le dice que levante `image_free` cuando la imagen esté de verdad fuera de
  pantalla.
- *submit* **espera** a `image_free` (en la etapa de transfer), **levanta** `render_done`
  para esa imagen, y enciende la fence cuando terminan los comandos.
- *present* **espera** a `render_done`.
El reinicio va después de acquire para que una vuelta temprana por out-of-date nunca deje la
fence apagada sin trabajo que la encienda.

**A-5.4** La validación informa de que la imagen presentada está en el layout equivocado —
más o menos "`vkQueuePresentKHR`: image is in `VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL`,
expected `VK_IMAGE_LAYOUT_PRESENT_SRC_KHR`" (`VUID-VkPresentInfoKHR-pImageIndices-01430`).
Sin validación el resultado es **indefinido**: a menudo se ve bien (muchos drivers de
escritorio usan la misma disposición de memoria para los dos), a veces sale basura, negro o
el fotograma anterior. "En mi máquina se veía bien" es exactamente como este bug llega a la
defensa en otra GPU.

**A-5.5** Con un fotograma en vuelo la CPU graba el fotograma N, lo envía, y al principio del
N+1 **espera a la fence hasta que la GPU ha terminado el fotograma N** — se queda ociosa
mientras la GPU dibuja, y luego la GPU se queda ociosa mientras la CPU graba. Un segundo
juego de objetos (command buffer, fence, `image_free`) deja que la CPU grabe el fotograma N+1
en el *otro* command buffer mientras la GPU sigue dibujando el N, así que las dos están
ocupadas. Solo espera cuando se adelanta dos fotogramas enteros.

## Sección 6

**A-6.1** El vertex shader se ejecuta **3 veces** — una por esquina — y decide *dónde cae
cada esquina* (`gl_Position`) más qué valores lleva (el color). El fragment shader se ejecuta
**unas 1000 veces** — una por píxel cubierto — y decide *el color de ese píxel*. Esa
proporción es la razón de que el trabajo caro por píxel vaya en el vertex shader cuando se
puede: tres ejecuciones en vez de mil.

**A-6.2** Una **mezcla más o menos a partes iguales de rojo, verde y azul — un blanco
grisáceo** (alrededor de un tercio de cada). Ninguno de tus programas lo mezcló: el
**rasterizador** (función fija, la parte de la GPU) interpoló los tres colores de las
esquinas, ponderados por distancia, y le pasó el valor mezclado a tu fragment shader, que
solo lo escribió. (Con una swapchain sRGB, la mezcla se ve más clara de lo que sugeriría un
tercio — § 4 / § 5.)

**A-6.3** **Abajo.** El eje y de OpenGL apunta hacia arriba, la y de NDC de Vulkan hacia
abajo, así que +0.5 está a medio camino hacia el borde *inferior*. Todo el triángulo sale
boca abajo comparado con la captura del tutorial. Arreglo: cambiar el signo de y (o, más
adelante, poner la inversión en la matriz de proyección, § 10).

**A-6.4** Dos cualesquiera de:
- **Formato del colour attachment** en el rendering info del pipeline ← el `format` de la
  swapchain. Mal → error de validación en `begin_rendering`/draw; sin validación,
  indefinido (basura o nada).
- **Viewport y scissor** (puestos al grabar) ← el `extent` de la swapchain. Tamaño
  equivocado → el triángulo aplastado en una esquina (viewport) o recortado (scissor) — el
  aspecto de "un cuarto de la ventana" de Retina de § 4.
- **Render area** en `begin_rendering` ← el extent. Demasiado pequeña → solo se limpia y se
  dibuja parte de la imagen.

**A-6.5** **Sí.** Guardar cambia un archivo en `scop/shaders/`; `cargo run` ve que el build
script pidió volver a ejecutarse cuando cambie esa carpeta
(`cargo:rerun-if-changed=shaders`), así que ejecuta `build.rs`, que ejecuta `glslc` y escribe
un `.spv` nuevo en `OUT_DIR`. El `include_bytes!` de `main.rs` depende de ese archivo, así que
scop se recompila con los bytes nuevos incrustados, y la ventana muestra un triángulo rojo.
Si `glslc` informa de un error en tu GLSL, el *build* falla con ese mensaje — nunca obtienes
un binario desfasado.

## Sección 7

**A-7.1** Un **`VkBuffer`** es una descripción — "N bytes, usables como vertex buffer y como
destino de copia" — sin almacenamiento. Un **`VkDeviceMemory`** es un bloque de memoria real
reservado, de un tipo de memoria. Enlazas el buffer a (parte de) la memoria. Están separados
para que el "qué" y el "dónde" se elijan de forma independiente: le preguntas al buffer qué
tipo de memoria necesita, eliges un tipo, y — en engines más grandes — metes muchos buffers
en una reserva grande, porque el número de reservas está limitado y cada una es lenta.

**A-7.2** Con index buffer: **4 vértices, 6 índices** (dos triángulos × 3). Sin él: **6
vértices**, dos de ellos duplicados. Para un cuadrado, el ahorro es poco. Para una tetera,
cada esquina la comparten unos seis triángulos, así que sin índices guardarías cada esquina
~6 veces: los índices reducen los datos de vértices más o menos en ese factor, y la GPU puede
reutilizar vértices ya procesados en vez de volver a ejecutar el vertex shader para las
copias.

**A-7.3** El submit vuelve enseguida; la copia se ejecuta en la GPU **más tarde**. Destruir
el staging buffer (y liberar su memoria) inmediatamente significa que la GPU puede copiar
desde memoria ya liberada — **comportamiento indefinido**: vértices basura, una forma
corrupta o un crash. La validación informa de que el buffer (o la memoria) se destruye
mientras **sigue en uso** por un command buffer pendiente. Arreglo: esperar a la fence de la
subida (o a la cola) antes de destruirlo.

**A-7.4** (a) **Staging:** memoria `HOST_VISIBLE` (la CPU debe mapearla y escribirla) y
`HOST_COHERENT` (para que las escrituras sean visibles a la GPU sin flush manual); uso
`TRANSFER_SRC` (se copia *desde* él). (b) **Vertex buffer final:** memoria `DEVICE_LOCAL`
(la más rápida para la GPU, se lee cada fotograma); uso `VERTEX_BUFFER` (se enlaza como
entrada de vértices) y `TRANSFER_DST` (se copia *hacia* él).

**A-7.5** La **forma sigue siendo correcta** (las posiciones se siguen leyendo desde el
offset 0), pero los **colores están mal**: el "color" de cada esquina se lee de los bytes
0..12 — su x, su y, y su valor real de rojo. Así que el color pasa a ser (x, y, rojo). Las
coordenadas negativas dan valores negativos, que acaban como 0 en la imagen, así que las
esquinas de la izquierda o de arriba pierden rojo/verde. Ningún error en ninguna parte — la
GPU lee los bytes exactamente como dice el albarán.

## Sección 8

**A-8.1** El **engine es dueño del bucle principal** (a través del bucle de eventos de
`winit`) y **llama a scop**: `setup` una vez cuando la GPU está lista, `update` con la
entrada y el delta time cada fotograma, `draw` para recoger la draw list del fotograma. scop
nunca ejecuta un bucle ni llama a "renderiza ahora"; solo responde cuando le llaman. Esa
inversión — el framework llama a la app, no la app a la librería — es la inversión de
control.

**A-8.2** **Engine:** (b) la swapchain, (d) el cargador de SPIR-V, (f) traducir los códigos
de tecla de `winit` — todos los proyectos los necesitan sin cambios. **scop:** (a) la ruta
del `.obj`, (c) el color del fondo, (e) el GLSL de los shaders de scop, (g) qué tecla activa
la textura — decisiones que toma este proyecto. (El *parser* de `.obj` es del engine —
ft_newton también carga modelos — pero *qué archivo* es elección de scop.)

**A-8.3** Que scop soltara la `Mesh` ejecutaría su `Drop`/destroy y liberaría los buffers
mientras hasta dos fotogramas en la GPU podrían seguir leyéndolos — **use-after-free en la
GPU**: la validación informa de buffers destruidos mientras están en uso; sin ella, basura o
un crash. Con un handle, el **engine conserva el ownership**: scop solo tiene un número, y el
engine destruye las meshes en su propio `Drop`, después de `device_wait_idle`. scop no puede
destruir lo que no posee.

**A-8.4** Con el `KeyCode` de `winit` en el trait, **todas las apps** que nombren una tecla
renombrada — scop, ft_newton, ft_vox — dejan de compilar y hay que editarlas, aunque a
ninguna le importe `winit`. Con el `Key` propio del engine, solo cambia **un sitio**: la
traducción del engine de códigos de `winit` a `Key`. Las apps ni se enteran. Eso es lo que
compra el acoplamiento débil.

**A-8.5** **`pub`:** `Vertex` (las apps construyen datos de mesh con él), `MeshId` (las apps
lo reciben y lo devuelven), `App` (las apps lo implementan). **`pub(crate)` o privado:**
`Buffer` (la memoria de GPU es cosa del engine; las apps reciben `MeshId`s en su lugar),
`find_memory_type` (un helper interno de `Buffer`), `GpuContext::draw_frame` (el engine lo
llama desde su propio bucle; una app que lo llamara directamente se saltaría el orden del
fotograma).

## Sección 9

**A-9.1** Una matriz de 3×3 puede rotar y escalar pero **no puede mover** (trasladar) nada —
no tiene dónde poner "suma 3 a x". Añadir una 4ª fila y columna da ese hueco, y un 4º número
`w` en cada vector. **Los puntos llevan `w = 1`, las direcciones `w = 0`.** La parte de
traslación de la matriz se multiplica por `w`, así que los puntos se mueven y las
direcciones no — "arriba" sigue siendo "arriba" esté donde esté el modelo. (La misma `w`
lleva más adelante la distancia para la división de perspectiva.)

**A-9.2** Se lee de derecha a izquierda: primero trasladar a (3, 0, 0), *luego* rotar
alrededor del origen — así que el modelo **da vueltas en círculo alrededor del origen**, a 3
unidades, como una puerta sobre su bisagra, en vez de girar sobre sí mismo. Seguramente
querías que girara en su sitio en (3, 0, 0): `translation(3, 0, 0) × rotation_y(angle)` —
girar primero (en el origen), luego mover.

**A-9.3** `model = rotation_y(angle) × translation(−2, −1, 0)` — de derecha a izquierda:
primero llevar el centro (2, 1, 0) al origen, luego rotar alrededor del origen. (Para
hacerlo girar en su sitio de vuelta en (2, 1, 0): `translation(2, 1, 0) ×
rotation_y(angle) × translation(−2, −1, 0)`.) Esto es exactamente el "rotar sobre su
centro" del subject.

**A-9.4** Más o menos **la mitad de tamaño** (en ancho y en alto — un cuarto del área). La
matriz de proyección pone la distancia de cada punto en `w` (2 y 4), y la **división de
perspectiva** de la GPU divide x e y entre `w` tras el vertex shader. El doble de lejos →
dividido entre el doble → la mitad de grande.

**A-9.5** GLSL ve la matriz **traspuesta**: filas y columnas intercambiadas. La traslación
(tx, ty, tz), que va en la 4ª columna, cae en la **4ª fila** — la fila que calcula `w`. Así
que en vez de mover el modelo, cambia la `w` de cada punto según su posición, y después la
división de perspectiva escala los puntos de forma desigual: el modelo se deforma, se
invierte, sale disparado de la pantalla o desaparece. Las rotaciones salen girando al revés.
No se informa de ningún error en ninguna parte — por eso el orden de almacenamiento se fija
una vez y se prueba.

## Sección 10

**A-10.1** La **cámara** (`projection × view`) va en un **uniform buffer**, alcanzado a
través de un descriptor set: cambia como mucho una vez por fotograma y es la misma para
todos los objetos, así que se escribe una vez y la comparten todos los draws. La matriz de
**modelo** va en un **push constant**: es distinta para cada objeto, y los push constants
viajan dentro del command buffer, así que cada draw puede llevar su propio valor sin coste
extra. (Las dos cabrían en 128 bytes de push constants, pero eso gasta todo el espacio
garantizado, y la maquinaria de descriptors hace falta en § 14 de todas formas.)

**A-10.2** Mientras la GPU sigue leyendo la cámara del fotograma N (slot A), la CPU escribe
la cámara del fotograma N+1 (slot B) en **el mismo buffer**. El fotograma N puede acabar
dibujándose en parte o del todo con la cámara del fotograma *siguiente*: saltos o tirones
ocasionales de un fotograma, sobre todo visibles cuando la cámara se mueve rápido. Es una
carrera, así que puede verse bien en una máquina y no en otra. La validation layer
normalmente no informa de **nada**: cada llamada es válida por sí sola; el problema es el
*momento*. Un buffer por slot, escrito tras la espera de la fence de ese slot, lo arregla.

**A-10.3** El `1.0` es `w`: la esquina es un **punto**, así que la traslación de la matriz de
modelo se le aplica (§ 9). Se lee de derecha a izquierda: primero `push.model` coloca y gira
la esquina en el mundo, luego `camera.view_proj` aplica la vista (cámara) y después la
proyección. O sea modelo → vista → proyección, es decir `P × V × M × esquina`.

**A-10.4** Un descriptor **apunta al** buffer; no guarda una copia de los datos. El shader
lee los bytes que haya en el buffer *cuando la GPU ejecuta el draw*, así que basta con
cambiar el contenido del buffer. Solo hace falta una nueva escritura de descriptor si el set
debe apuntar a un **recurso distinto**: otro buffer, otro offset o tamaño, o (§ 14) otra
textura.

**A-10.5** El cubo se ve **estirado a lo ancho** (el doble de ancho de lo que debería),
porque la proyección sigue usando el ancho ÷ alto viejo. Debería salir del **tamaño actual
de la imagen de la swapchain cada fotograma** (`frame.aspect()`), para que siempre esté bien
tras redimensionar.

## Sección 11

**A-11.1** Un **valor de profundidad** por píxel: a qué distancia está (0 = plano cercano,
1 = plano lejano) lo que hay dibujado en ese píxel. Se reinicia a **1.0** — el plano lejano,
"lo más lejos posible" — para que el primer fragmento dibujado en cualquier sitio esté
siempre más cerca (`LESS`) y le gane al fondo vacío.

**A-11.2** El buffer nunca cambia de su 1.0 de limpieza, así que **todos** los fragmentos
pasan el test (todo es menor que 1.0) — exactamente como si no hubiera depth buffer: gana el
último triángulo dibujado, y las caras traseras vuelven a verse en algunos ángulos. El test
necesita profundidades recordadas contra las que comparar; sin escrituras no hay ninguna.

**A-11.3** **Solo el fondo.** Toda profundidad está entre 0 y 1, y nada es *mayor* que el
1.0 de limpieza, así que todos los fragmentos fallan y se descartan.

**A-11.4** Los números de profundidad solo hacen falta **durante** el fotograma, para
decidir qué fragmento gana cada píxel. Cuando termina el renderizado, nadie los lee — el
siguiente fotograma los limpia a 1.0 de todos modos. La imagen de color es distinta: se
muestra en pantalla, así que hay que conservarla (`STORE`). `DONT_CARE` deja que la GPU se
ahorre escribir los números de profundidad de vuelta a memoria, lo que ahorra ancho de
banda.

**A-11.5** **Z-fighting.** La precisión de profundidad se amontona cerca del plano cercano;
con un `near` diminuto, las superficies más lejanas reciben casi los mismos números de
profundidad, así que el redondeo elige al ganador píxel a píxel: rayas que parpadean. Se ve
primero **donde dos caras están juntas y lejos de la cámara** (aristas donde se encuentran
caras, la parte lejana del modelo).

## Sección 12

**A-12.1** **`v`** (la posición de una esquina) y **`f`** (una cara, que lista esquinas por
número). Todo lo demás se salta: comentarios (`#`), `o` / `g` (nombres de objeto y de
grupo), `s` (suavizado), `mtllib` / `usemtl` (materiales), `vt` / `vn` (coordenadas de
textura, normales), `l` / `p` (líneas, puntos), líneas vacías.

**A-12.2** Esquinas 16, 2, 3, 17 → desde 0: 15, 1, 2, 16. Abanico desde la primera esquina:
**(15, 1, 2)** y **(15, 2, 16)**.

**A-12.3** Cada cara uniría la esquina *siguiente* en vez de la correcta, así que el logo se
convierte en una maraña de triángulos equivocados y puntiagudos. Una cara que use la esquina
42 pediría `positions[42]`, pero solo existen los índices 0..41: un panic (index out of
bounds) si no se comprueba, o — con la comprobación de rango del parser — un error limpio
con el número de línea.

**A-12.4** Algunos triángulos del abanico pueden cubrir área **fuera** de la L (cruzando la
esquina interior), así que la cara parece tener un triángulo extra relleno, y algunas partes
pueden solaparse. Vale para los archivos obligatorios; tratar bien las caras cóncavas es el
bonus de § 17.

**A-12.5** `model = rotation_y(angle) × translation(−centre)` — de derecha a izquierda:
llevar el centro al origen, luego girar. El centro es el centro de la **bounding box**:
`(min + max) / 2` de todas las esquinas, calculado por el parser.

## Sección 13

**A-13.1** Los grises **se mezclan entre sí**: una esquina compartida solo puede tener un
color, así que toda cara que la toca recibe ese color en esa esquina, y el rasterizador
mezcla entre esquinas. Verías degradados de gris suaves por todo el logo en vez de caras
planas distintas, sin aristas visibles. (Qué gris acaba teniendo cada esquina depende de qué
cara la escribió la última.)

**A-13.2** **18 960** vértices (6320 triángulos × 3 esquinas cada uno). Los índices son
simplemente `0, 1, 2, 3, …, 18959`: cada vértice se usa exactamente una vez.

**A-13.3** Las flechas son **continuas**: el movimiento debe ocurrir cada fotograma que la
tecla está abajo, así que `held`. Space es un **interruptor**: una pulsación = un cambio, así
que `pressed` (true solo en el fotograma en que baja la tecla). Con `held`, el giro se
encendería y apagaría cada fotograma mientras la tecla está abajo — un encendido/apagado que
parece aleatorio según cuántos fotogramas la mantuviste.

**A-13.4** Se lee de derecha a izquierda: centrar → **mover** → girar. Ahora el giro ocurre
*después* del movimiento, así que la propia posición se gira: `rotation_y(90°)` convierte +x
en −z. Mantener → aumenta position.x, que acaba siendo moverse **alejándose de la cámara**
(el modelo se hace más pequeño) en vez de a la derecha — y cambiar el ángulo hace que el
modelo dé vueltas alrededor del origen. La tarjeta de traslación debe ser la más a la
izquierda para un movimiento según los ejes de la pantalla (espacio del mundo).

**A-13.5** **`dt`**: por segundo, así que la misma velocidad a cualquier tasa de fotogramas.
**`radius`**: en el tamaño propio del modelo, así que un logo diminuto y una tetera grande se
mueven ambos a un ritmo razonable. **`0.2`**: la velocidad elegida — una quinta parte del
tamaño del modelo por segundo.

## Sección 14

**A-14.1** Dos números por esquina que dicen **en qué punto de la imagen** está esa esquina:
**U** en horizontal (0 = borde izquierdo, 1 = borde derecho) y **V** en vertical (0 =
arriba, 1 = abajo). La GPU los mezcla a lo largo de cada triángulo para que cada píxel sepa
su punto, y luego busca el color ahí. scop **se las inventa** con mapeo plano: proyectar a lo
largo del eje más delgado de la bounding box y escalar las otras dos coordenadas a 0..1.

**A-14.2** **Rayas largas** con los colores del borde de la imagen. A lo largo de una cara
lateral delgada, el eje proyectado (x) cambia pero U y V casi no — cada punto de esa cara cae
casi en la misma franja fina de la imagen, así que esos pocos texels se emborronan por toda
la cara. Ese es el "estiramiento visible" que el bonus de § 18 te pide quitar.

**A-14.3** La imagen sale **boca abajo** (reflejada de arriba abajo): el cargador trata la
primera fila guardada — la de abajo de la imagen — como la de arriba.

**A-14.4** La CPU la escribe **una vez**, la GPU la lee cada fotograma → staging a
`DEVICE_LOCAL` (lecturas más rápidas), igual que las meshes. `UNDEFINED →
TRANSFER_DST_OPTIMAL`: preparar la imagen para que **la escriba la copia**.
`TRANSFER_DST_OPTIMAL → SHADER_READ_ONLY_OPTIMAL`: tras la copia, prepararla (y hacer
visibles las escrituras de la copia) para las **lecturas del fragment shader**.

**A-14.5** **Desvaídos / demasiado pálidos.** Los bytes del archivo ya están codificados en
sRGB (detalle extra en los tonos oscuros). `UNORM` le dice a la GPU que son brillo sin más,
así que se salta la decodificación; la swapchain sRGB luego los codifica una *segunda* vez a
la salida. Doble codificación = todo más brillante y pálido de lo que es de verdad la
imagen.

## Sección 15

**A-15.1** `t = 0` → **`a`**; `t = 0.5` → **a medio camino entre** `a` y `b`; `t = 1` →
**`b`**. En scop: `a` = el gris de la cara, `b` = el color de la imagen en ese punto, `t` = el
blend factor (progreso suavizado del fundido).

**A-15.2** Para que el fundido pueda **moverse poco a poco** e **invertirse sin salto**. La
tecla solo dice hacia dónde ir; cada fotograma el blend factor da un pasito del tamaño de
`dt` hacia ese objetivo. Ponerlo directamente haría que la imagen apareciera o desapareciera
de golpe — el corte brusco que prohíbe el subject.

**A-15.3** El progreso deja de subir en 0.4 y empieza a bajar desde **0.4** — sin salto —
llegando a 0 (todo gris) unos 0.4 × 0.6 s ≈ 0.24 s después, y se queda ahí. La imagen se
desvanece con suavidad desde donde estaba.

**A-15.4** Un **error de validación al crear el pipeline**: el fragment shader usa bytes de
push constant que ningún rango declara para la etapa `FRAGMENT`. (Si lo ejecutaras igualmente,
el valor de blend que lee el fragment shader sería indefinido.)

**A-15.5** Cambia el **ritmo**. El lineal va a velocidad constante, así que arranca y para con
un "tirón" repentino. smoothstep empieza despacio, acelera en medio y vuelve a frenar al final,
así que la diferencia se ve **justo al principio y justo al final** del fundido — entra y sale
con suavidad en vez de arrancar y parar de golpe.

## Sección 16

**A-16.1** Las imágenes de la swapchain están hechas para **un tamaño de ventana**; tras
redimensionar ya no encajan, y acquire/present informan de out-of-date (o suboptimal). La
**imagen de profundidad** debe reconstruirse con ella, porque tiene que tener exactamente el
tamaño de la swapchain (un hueco de profundidad por píxel). Los pipelines no necesitan
reconstruirse porque el viewport y el scissor son **dynamic state**, puestos cada fotograma a
partir del extent actual (§ 6) — y los formatos no cambian.

**A-16.2** Puede que la GPU siga usando las imágenes, views y semáforos de la swapchain vieja
para fotogramas ya enviados. Destruirlos debajo de ella es un **error de validación** (objeto
en uso) y puede provocar un crash o la pérdida del dispositivo. `device_wait_idle` antes
garantiza que nada está en uso.

**A-16.3** Comprueba **`cargo`/Rust**, **`glslc`** y el **loader de Vulkan** (y MoltenVK en
macOS). Rust siempre se puede instalar en tu carpeta personal (rustup, sin admin). En el Linux
del campus, `glslc` y Vulkan son **paquetes del sistema que necesitan permisos de
administrador**, así que el script no puede instalarlos — informa exactamente de lo que falta y
del comando para conseguirlo. El subject dice *comprobar e instalar si es posible*: comprobar
es obligatorio, instalar solo donde se pueda.

**A-16.4** (1) Primera línea en cursiva: *This project has been created as part of the 42
curriculum by luferna3.* (2) Una sección **Description** (objetivo + visión general breve).
(3) Una sección **Instructions** (compilación, instalación, ejecución). (4) Una sección
**Resources** con referencias clásicas **y** cómo se usó la IA — para qué tareas y qué partes.

**A-16.5** Las líneas `vt` y `vn` se **saltan** (scop hace sus propias UVs y no ilumina).
`f 1/1/1 2/2/2 3/3/3` usa solo la parte antes de la primera `/`, así que son las esquinas 1, 2,
3 → desde 0: 0, 1, 2. La cara de 5 esquinas se **triangula en abanico** en 3 triángulos, todos
con el mismo gris (un número de cara) — correcto si es convexa; una cóncava puede mostrar
triángulos sueltos (bonus de § 17).

## Sección 17

**A-17.1** Una **oreja** son tres esquinas consecutivas cuyo triángulo queda entero **dentro**
del polígono: la esquina del medio apunta hacia fuera (giro convexo) y ninguna otra esquina
está dentro del triángulo. Cortarla solo quita área que pertenece a la cara, y lo que queda
sigue siendo un polígono válido con una esquina menos — así que repetirlo siempre acaba en una
triangulación correcta.

**A-17.2** La **comprobación de área**: para una cara plana, las áreas de los triángulos de una
triangulación correcta suman exactamente el área de la cara. Un abanico desde una mala esquina
de una L incluye un triángulo que se sale de la forma, así que sus áreas suman **más** que el
área de la cara (y se solapan).

**A-17.3** Los tests de oreja ("¿gira hacia fuera?", "¿hay una esquina dentro de este
triángulo?") son sencillos y fiables en **2D**; aplanar según la normal de la cara conserva su
contorno y su sentido de giro. La **normal de Newell** usa todas las esquinas, así que da la
orientación media real incluso para caras cóncavas o dobladas; el producto vectorial de las dos
primeras aristas solo ve tres esquinas — si la del medio es una esquina **reflex** (entrante),
apunta *al revés* y todos los tests saldrían invertidos.

**A-17.4** Deja de buscar orejas y **hace un abanico con las esquinas que quedan** (la
alternativa). Eso garantiza que el bucle termina (no se cuelga con un archivo roto) y aun así
dibuja algo razonable para esa cara, mientras el resto del modelo carga con normalidad — mejor
que rechazar un modelo entero por una cara mal formada.

**A-17.5** Demostrar que está **en el archivo**: `teapot.obj` y `teapot2.obj` muestran los
mismos anillos desde el mismo ángulo, y contar aristas da ~100 **aristas abiertas** alrededor
del borde de la tapa en los dos archivos (aristas que usa un solo triángulo — las superficies
de la tapa y del cuerpo nunca se unen). scop dibuja exactamente lo que describe el archivo, por
los dos lados, con la profundidad correcta.

## Sección 18

**A-18.1** § 14 proyectaba la imagen a lo largo de **un** eje para todo el modelo. Las caras que
van *a lo largo* de ese eje (los lados delgados del logo) quedan de canto respecto a la
proyección: cada punto de ellas cae casi en la misma franja fina de la imagen, que se emborrona
por toda la cara. El box mapping le da a **cada cara** la proyección que tiene más de frente
(según su normal de Newell), así que ninguna cara queda nunca de canto respecto a su
proyector.

**A-18.2** Dividir cada una entre su propio lado aprieta la imagen para que encaje en la caja
en las dos direcciones: en un objeto largo y delgado, una dirección recibe muchos más texels por
unidad que la otra, así que la imagen **se estira**. Una escala compartida mantiene la misma
densidad de texels en horizontal y en vertical — los cuadrados siguen siendo cuadrados.

**A-18.3** Qué imagen lee un draw lo decide el **set de textura enlazado para ese draw** (set 1,
§ 14). Una mesh dibujada una vez solo puede tener un set enlazado, así que una imagen. Dividir el
modelo en una mesh por objeto, cada una dibujada con su propio `draw_textured`, deja que cada una
enlace su propia imagen.

**A-18.4** Un **error de validación al crear el pipeline**: `POLYGON_MODE_LINE` requiere que la
feature `fillModeNonSolid` esté **activada en el dispositivo**, no solo soportada por la GPU. (Sin
validación, el resultado es indefinido — quizá triángulos sólidos, quizá un crash.)

**A-18.5** Cambia la **perspectiva**. Acercarse la exagera: las partes más cercanas a la cámara
crecen mucho más que las lejanas, así que el modelo se ve más profundo y más deformado. Hacer zoom
deja la cámara donde está y solo recorta la vista, así que las partes cercanas y lejanas crecen
por igual: el modelo se ve **más plano**, como con un teleobjetivo. (Mismo tamaño en pantalla,
distinta forma.)

## Sección 19

**A-19.1** El **techo de vsync**: scop presenta con FIFO, que espera al refresco de la pantalla,
así que no puede mostrar más fotogramas de los que dibuja la pantalla (120 Hz → 120 fps). El
número te dice que scop va al ritmo de la pantalla; **no** te dice a qué velocidad podría ir la
GPU, ni qué modelo es más pesado. Para eso, compara frame times en un build release (o un present
mode sin techo).

**A-19.2** (1) `1 ÷ dt` cambia cada fotograma — pequeñas variaciones de tiempo hacen que salte
entre valores que no se pueden leer. Una media sobre 0.5 s es estable y más fiel. (2) El título se
pondría cada fotograma, lo que es lento en algunos sistemas (macOS redibuja la barra de título);
promediar hace que cambie dos veces por segundo, y solo cuando el texto es distinto.

**A-19.3** Hasta dos fotogramas siguen en vuelo (§ 5), y se grabaron con los buffers del modelo
viejo. Destruirlos ahora significa que la GPU puede leer memoria liberada mientras termina esos
fotogramas: un **use-after-free en la GPU**, en los pocos milisegundos siguientes a soltar el
archivo. Con la validación activada, sí: informa de un buffer **destruido mientras sigue en uso**
por un command buffer enviado. Sin validación, es indefinido — basura, un crash o nada visible.
Arreglo: esperar (`device_wait_idle`) antes de destruir.

**A-19.4** (1) El archivo está **del revés**: sus caras listan las esquinas en sentido horario
vistas desde fuera. (2) El **delante/detrás de scop está intercambiado**: el `front_face` del
pipeline no coincide con el winding tras la inversión de y de la proyección. Prueba: abre un modelo
de confianza (la tetera o el cubo) — si también sale rojo por fuera, la causa es el ajuste de
scop, no el archivo.

**A-19.5** La profundidad guardada **no es lineal**: la tarjeta de perspectiva aprieta las
distancias lejanas hacia 1, guardando la mayor parte de la precisión cerca del plano cercano. El
plano cercano de scop es diminuto (1 % del tamaño del modelo), así que todo el modelo cae alrededor
de 0.99–1.0: casi el mismo blanco en todas partes. scop usa la **distancia lineal** — la `w` del
vertex shader — repartida sobre el rango propio del modelo de más cercano a más lejano, así que se
usa toda la escala de blanco a oscuro.

## Sección 20

**A-20.1** Orbitar mueve solo la cámara: el modelo y la luz se quedan donde están, así que **las
mismas caras siguen iluminadas** — das la vuelta y ves el lado iluminado y luego el oscuro. Girar el
modelo mueve sus caras respecto a la luz, así que **cambia qué caras están iluminadas** mientras gira.

**A-20.2** La cámara **se da la vuelta** de golpe en un fotograma al pasar por la vertical: "arriba"
(+y) y la dirección de la vista se convierten en la misma línea, así que deja de estar definido cuál
es arriba, y al otro lado sale invertido. El límite (±89°) la para justo antes.

**A-20.3** La luz parece **pintada**: las caras que estaban brillantes siguen brillantes y las oscuras
siguen oscuras, gire como gire el modelo — el shader sigue creyendo que cada cara apunta adonde
apuntaba al principio.

**A-20.4** Las caras de espaldas a la luz recibirían **cero** — negro puro, invisibles contra el fondo
oscuro, y el contorno del modelo desaparecería por su lado oscuro. El ambiente mantiene cada cara al
menos tenuemente visible.

**A-20.5** Una esquina guarda **una** normal. Con esquinas propias, las esquinas de cada triángulo
llevan la normal de su cara, así que toda la cara tiene un brillo y las aristas siguen nítidas. Las
esquinas compartidas pertenecen a varias caras a la vez, así que necesitarían una normal
**promediada** — el brillo se mezclaría a través de las aristas: **smooth shading**, aspecto
redondeado, incorrecto para las esquinas marcadas del logo.

## Sección 21

**A-21.1** Vulkan solo acepta **SPIR-V** (§ 6), nunca texto GLSL. Algo tiene que traducir antes el
`.frag` a SPIR-V — al compilar es `build.rs`; para la recarga en caliente es el mismo `glslc`,
ejecutado con scop en marcha.

**A-21.2** Para que un fallo no cueste nada: si el shader nuevo no compila o no se puede construir el
pipeline, **los pipelines viejos siguen ahí** y scop sigue dibujando con ellos. Destruir primero
dejaría a scop sin nada con lo que dibujar (la misma regla que al soltar un modelo en § 19).

**A-21.3** **Pantalla: sin cambios** — los shaders viejos siguen funcionando. **Terminal: el error de
`glslc`**, con el archivo y la línea del problema. Arreglar y guardar → la siguiente comprobación ve
una fecha de modificación más nueva, compila, cambia el pipeline e imprime "shaders reloaded".

**A-21.4** Rompe el **contrato del shader**: el shader ahora espera una entrada de vértice en la
location 4, pero la disposición de vértice del lado de Rust (§ 7) no tiene nada ahí — la validación
informa de la entrada que falta (sin validación, los valores son indefinidos). La recarga en caliente
no puede añadir datos que el lado de Rust no envía: hace falta un campo `tangent` en `Vertex`, un 5º
atributo, que scop rellene los datos — y recompilar y reiniciar.

**A-21.5** Un id de pipeline significa "el pipeline sólido (wireframe, puntos) de scop", no "esta
versión exacta": scop *quiere* que todos los draws siguientes usen el nuevo, sin actualizar sus
resguardos. Nada puede dibujar el viejo por error — se destruye solo cuando la GPU ha terminado con
él, y nadie guarda un id aparte para él. En § 19 no se podía reutilizar el hueco porque podría
aparecer una mesh *distinta* bajo un id viejo: forma equivocada, sin error. Aquí es la misma cosa,
actualizada.

## Sección 22

**A-22.1** Ahorra la **repetición**: grabar 512 juegos de comandos bind / push / draw en la CPU, y el
coste de arranque de la GPU para 512 draws — es un comando que dice "512 copias". **No** ahorra el
dibujo: la GPU sigue ejecutando el vertex shader para cada esquina de cada copia y rellenando cada
triángulo (512 × la mesh).

**A-22.2** **Demasiado grande para el bloque de push**: cabe más o menos una matriz (128 bytes
garantizados), y 512 matrices son 32 KB — un buffer que el shader lee como un array. **Uno por slot de
fotograma** porque la CPU reescribe la lista cada fotograma mientras puede que la GPU aún esté leyendo
la del fotograma anterior (§ 10): una lista compartida haría que las copias saltaran o parpadearan, en
silencio.

**A-22.3** Se dibuja con la **matriz de la primera copia de la multitud** en vez de la identidad:
reducido al tamaño de una copia, movido a una esquina de la cuadrícula y girando por su cuenta —
porque los draws normales leen la posición 0, y la posición 0 ya no es "no cambiar nada".

**A-22.4** No. El **techo de vsync** (§ 19): FIFO espera a la pantalla, así que el título no puede
mostrar más de 60 tanto si el fotograma costó 2 ms como 15 ms de trabajo real. La GPU está dibujando
512 × los triángulos de la tetera; para ver el coste, compara frame times sin el techo, o fíjate en si
el número baja *de* 60.

**A-22.5** El vertex shader gira cada normal con el modelo **completo** de esa copia — pósit × la
matriz propia de la copia — la misma tarjeta combinada que coloca sus esquinas. Así que las normales
de cada copia giran con ella, y el código de la luz sin cambios (§ 20) ve la dirección correcta para
cada cara de cada copia.

## Sección 23

**A-23.1** Seis imágenes cuadradas — las caras interiores de una caja — guardadas como una textura. Se
consulta con una **dirección** porque describe lo que hay *alrededor* de un punto, no a lo largo de una
superficie plana: la dirección dice "¿qué hay por ahí?", y la GPU elige la cara a la que apunta y el
punto dentro de ella. Toda dirección tiene respuesta, así que cubre todo el entorno.

**A-23.2** Se comportaría como una habitación, no como un cielo: al moverse la cámara en su órbita se
acercaría a una pared y se alejaría de otra, las paredes pasarían a velocidades distintas, y podrías
ver los **bordes y las esquinas** de la caja. Moviéndose con la cámara, solo girar cambia la vista —
como algo infinitamente lejano.

**A-23.3** **Solo el cielo.** Dibujada primero con la escritura de profundidad activada, la caja
escribe su profundidad (muy cercana) en toda la pantalla; el modelo está más lejos, así que falla el
test de profundidad (§ 11) en todas partes y nunca se dibuja. Con la escritura de profundidad
desactivada, la caja pinta el fondo pero deja la imagen de profundidad en "aquí no hay nada", así que
el modelo pasa el test y se dibuja encima — sea cual sea el tamaño de la caja.

**A-23.4** Cada cara es **plana** con una normal (flat shading, § 20), así que cada una actúa como su
propio espejo plano. Las caras vecinas apuntan en direcciones distintas, así que en su arista común la
dirección del rebote salta — el reflejo se rompe de golpe. Los cientos de caritas planas de la tetera
se convierten en cientos de espejitos: una bola de discoteca. Con normales suaves (promediadas) el
cromado sería suave.

**A-23.5** Para que todo cuente la misma historia: el lado brillante del modelo (luz de § 20) mira al
sol que ves en el cielo y en sus reflejos. Con el sol en otro sitio, el modelo estaría iluminado desde
un lado mientras el sol del cielo está en otro — parece falso enseguida, aunque no sepas decir por qué.

## Sección 24

**A-24.1** **scop.** Otro proyecto no querría caras grises sin cambios — existen porque el subject pide
grises distinguibles: una decisión, no un mecanismo. Lo que va al engine es el *mecanismo*: construir
una mesh por objeto con sus propias esquinas y normales planas, pidiendo el color mediante un
**callback** — el callback de scop devuelve sus grises.

**A-24.2** Porque la vuelta de la cámara sale de las propias matemáticas de cámara del engine (`look_at`
con un "arriba" fijo): es una propiedad de la cámara del engine, no de una app. Si cada app tuviera que
acordarse del límite, alguna lo olvidaría y la cámara se daría la vuelta al pasar por encima. El engine
es dueño de la protección (como un ajuste con un valor por defecto seguro).

**A-24.3** Escribe tus propios shaders (un **override**), crea un pipeline con ellos y dibuja con
`draw_with`, cuyos cuatro números significan lo que digan tus shaders. Reutilizar el pipeline por
defecto metería tus números en shaders que los leen como blend / modo de color / rango de profundidad
— colores equivocados, en silencio. Cada pipeline es dueño del significado de sus push params.

**A-24.4** Solo la app sabe cuáles de sus pipelines se construyeron con qué shaders, y en qué estilo
(relleno, wireframe, puntos, fondo). El engine solo ve pipelines y bytes. La mitad general — notar un
guardado y compilar — es igual para todo proyecto, así que se mueve; la correspondencia se queda con la
app.

**A-24.5** El "arriba" de la cámara está fijado como +y: en un mundo con +z arriba, orbitar sube por el
lado equivocado, el horizonte se inclina, y el límite de pitch protege el eje equivocado. El **eje de
arriba** (y el límite respecto a él) debería ser un ajuste, con +y por defecto para que scop no cambie.
