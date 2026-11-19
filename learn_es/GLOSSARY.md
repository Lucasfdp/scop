# Glosario — scop

Términos en el orden en que se introducen. Primero en lenguaje llano, luego con precisión.
Los identificadores conservan su nombre en inglés.

---

## Sección 1 — Rust

### ownership
**Llano:** cada valor tiene exactamente una variable responsable de limpiarlo (propiedad).
**Preciso:** cuando el dueño sale de su ámbito, el valor se destruye (se ejecuta su `Drop`,
se libera la memoria). La propiedad se puede mover pero nunca duplicar en tipos no `Copy`.
**Introducido en:** § 1

### move
**Llano:** dar un valor a otro; después ya no puedes usarlo.
**Preciso:** la asignación o el paso por valor transfiere la propiedad en tipos no `Copy`;
la variable origen queda inválida (`E0382` si se usa). Los tipos `Copy` se copian bit a bit.
**Introducido en:** § 1

### borrow
**Llano:** prestar un valor sin darlo.
**Preciso:** `&T` (compartido, solo lectura, cualquier número) o `&mut T` (exclusivo,
lectura-escritura, exactamente uno, sin préstamos compartidos vivos a la vez). Lo comprueba
en compilación el borrow checker.
**Introducido en:** § 1

### lifetime
**Llano:** cuánto puede vivir un préstamo — nunca más que aquello a lo que apunta.
**Preciso:** una región en tiempo de compilación, normalmente deducida, escrita `'a` cuando
se anota. Impide referencias colgantes.
**Introducido en:** § 1

### Option
**Llano:** "quizá un valor". El sustituto de NULL en Rust.
**Preciso:** `enum Option<T> { Some(T), None }`. Debes tratar `None` para llegar al valor.
**Introducido en:** § 1

### Result
**Llano:** "un valor o un error". El sustituto en Rust de los códigos de retorno y `errno`.
**Preciso:** `enum Result<T, E> { Ok(T), Err(E) }`.
**Introducido en:** § 1

### ? operator
**Llano:** "si esto ha fallado, devuelve el error desde mi función ya; si no, dame el valor."
**Preciso:** retorna antes `Err(From::from(e))`; solo se puede usar en funciones que
devuelven `Result` u `Option`. Los tipos de error deben coincidir o convertirse con `From`.
**Introducido en:** § 1

### trait
**Llano:** un conjunto de funciones que un tipo promete ofrecer — una interfaz.
**Preciso:** p. ej. `Debug`, `Clone`, `Copy`, `Drop`, `From`. Se implementa con `impl Trait for Type`.
**Introducido en:** § 1

### Drop
**Llano:** el destructor de Rust — se ejecuta solo cuando un valor sale de su ámbito.
**Preciso:** trait con `fn drop(&mut self)`. Los campos de un struct se destruyen en orden de
declaración; las variables locales, en orden inverso.
**Introducido en:** § 1
**Lo verás en:** cada envoltorio de Vulkan (§ 3 en adelante)

### RAII
**Significa:** Resource Acquisition Is Initialisation (adquirir el recurso es inicializar)
**Llano:** atar un recurso a la vida de un valor: crear el valor → obtener el recurso;
destruir el valor → liberarlo.
**Preciso:** un idiom de C++; en Rust se implementa con `Drop`. El nombre solo describe la
mitad de adquirir — la de liberar es la útil.
**Introducido en:** § 1

### crate
**Llano:** una librería o un ejecutable de Rust.
**Preciso:** la unidad de compilación; un crate librería (`lib.rs`) o binario (`main.rs`).
**Introducido en:** § 1

### Cargo workspace
**Llano:** varios crates en un repo, compilados juntos.
**Preciso:** un `Cargo.toml` raíz con `[workspace] members = [...]`; `target/` y
`Cargo.lock` compartidos. scop: `engine` (lib) + `scop` (bin).
**Introducido en:** § 1

### unsafe
**Llano:** un bloque donde asumes tú las comprobaciones que el compilador no puede hacer — como escribir C.
**Preciso:** permite desreferenciar punteros crudos, llamar `unsafe fn` (todas las llamadas de
Vulkan en `ash`), statics mutables, traits `unsafe` y campos de union. Las reglas de préstamo
siguen aplicando a las referencias.
**Introducido en:** § 1

### raw pointer
**Llano:** un puntero de C de toda la vida (puntero crudo).
**Preciso:** `*const T` / `*mut T`: sin seguimiento de propiedad ni de lifetime; puede ser
nulo o colgante. Crearlo es seguro; desreferenciarlo es `unsafe`.
**Introducido en:** § 1

### CString
**Llano:** un string con propiedad y terminado en nulo, la forma que esperan las APIs de C.
**Preciso:** `std::ffi::CString`; `.as_ptr()` da un `*const c_char` válido solo mientras el
`CString` viva.
**Introducido en:** § 1

### repr(C)
**Significa:** representation C (representación C)
**Llano:** "dispón este struct exactamente como lo haría C".
**Preciso:** `#[repr(C)]` fija el orden de los campos y el relleno según la ABI de C.
Necesario para datos que la GPU lee byte a byte (vértices, uniforms).
**Introducido en:** § 1

---

## Sección 2 — Cómo dibuja una GPU

### CPU
**Significa:** Central Processing Unit (unidad central de procesamiento)
**Llano:** el procesador principal — pocos núcleos rápidos y listos. Ejecuta tu código Rust.
**Introducido en:** § 2

### GPU
**Significa:** Graphics Processing Unit (unidad de procesamiento gráfico)
**Llano:** miles de núcleos simples que ejecutan todos a la vez el mismo programa pequeño sobre datos distintos.
**Preciso:** un procesador aparte al que mandas trabajo mediante una API; asíncrono respecto a la CPU.
**Introducido en:** § 2

### API
**Significa:** Application Programming Interface (interfaz de programación de aplicaciones)
**Llano:** el conjunto de funciones que te ofrece una librería o sistema.
**Introducido en:** § 2

### Vulkan
**Llano:** una API gráfica explícita y de bajo nivel: configuras todo, el driver no adivina nada.
**Preciso:** Khronos Group, 2016, sucesora de Mantle de AMD. Nombrada por Vulcano, dios de la forja.
**Introducido en:** § 2

### MoltenVK
**Significa:** "Molten" (fundido) + VK (Vulkan)
**Llano:** una capa que ejecuta Vulkan sobre Metal de Apple, porque macOS no tiene Vulkan nativo.
**Preciso:** implementa (casi todo) Vulkan como implementación de "portabilidad"; exige
`VK_KHR_portability_enumeration` al crear la instancia.
**Introducido en:** § 2

### rendering pipeline
**Llano:** la cadena de montaje fija que va de vértices a píxeles.
**Preciso:** vertex shader → primitive assembly → rasterización → fragment shader →
depth test/blending → framebuffer.
**Introducido en:** § 2

### vertex
**Llano:** una esquina, más los datos que lleve asociados (color, coordenadas de textura).
**Preciso:** plural *vertices*; la entrada por vértice del vertex shader.
**Introducido en:** § 2

### primitive
**Llano:** la forma más pequeña que dibuja la GPU — para ti, un triángulo.
**Introducido en:** § 2

### vertex shader
**Llano:** tu programa de GPU que se ejecuta una vez por vértice y decide dónde cae en pantalla.
**Preciso:** devuelve `gl_Position` en clip space, más los valores a interpolar.
**Introducido en:** § 2

### rasterisation
**Llano:** calcular qué píxeles cubre un triángulo (rasterización).
**Preciso:** etapa de función fija que produce fragmentos e interpola las salidas de los
vértices a lo largo del triángulo. De *raster*, la cuadrícula de líneas de las pantallas antiguas.
**Introducido en:** § 2

### fragment
**Llano:** un píxel candidato — "este triángulo cubre este píxel" — antes de saber si es visible.
**Introducido en:** § 2

### fragment shader
**Llano:** tu programa de GPU que se ejecuta una vez por fragmento y decide su color.
**Preciso:** DirectX lo llama pixel shader.
**Introducido en:** § 2

### framebuffer
**Llano:** la(s) imagen(es) en la(s) que se dibuja — color y, más adelante, profundidad.
**Introducido en:** § 2

### swapchain
**Llano:** 2–3 imágenes que se turnan: una en pantalla, otra dibujándose.
**Preciso:** `VkSwapchainKHR`, creada a partir de una surface (§ 4).
**Introducido en:** § 2

### NDC
**Significa:** Normalised Device Coordinates (coordenadas normalizadas de dispositivo)
**Llano:** una caja fija donde acaba la salida del vertex shader: −1..+1 a lo ancho de la ventana.
**Preciso:** clip space dividido por `w`. En Vulkan: x a la derecha, **y hacia abajo**,
profundidad 0..1 (OpenGL: y hacia arriba, profundidad −1..1).
**Introducido en:** § 2

### GLSL
**Significa:** OpenGL Shading Language (lenguaje de sombreado de OpenGL)
**Llano:** el lenguaje parecido a C en el que escribes shaders. Sirve para Vulkan pese al nombre.
**Introducido en:** § 2

### SPIR-V
**Significa:** Standard Portable Intermediate Representation, versión 5
**Llano:** bytes de shader compilado — Vulkan solo acepta esto, nunca texto GLSL.
**Preciso:** lo produce `glslc`; lo cargas tú en un `VkShaderModule` (§ 6).
**Introducido en:** § 2

### command buffer
**Llano:** una lista de la compra grabada con trabajo para la GPU, entregada de una vez.
**Preciso:** `VkCommandBuffer`, se graba en la CPU y se envía a una cola (§ 5).
**Introducido en:** § 2

### queue
**Llano:** la bandeja de entrada de la GPU para command buffers. Enviar vuelve al instante.
**Preciso:** `VkQueue`; el trabajo termina de forma asíncrona, señalado con fences/semaphores.
**Introducido en:** § 2

### winit
**Significa:** window initialisation (inicialización de ventanas)
**Llano:** el crate de Rust que crea la ventana y entrega eventos de teclado/ratón/redimensionado — tu mlx.
**Preciso:** desde 0.30, se implementa `ApplicationHandler`; las ventanas se crean en `resumed`.
**Introducido en:** § 2

### event loop
**Llano:** el bucle que espera a que pasen cosas y llama a tu código para cada una — como `mlx_loop` con hooks.
**Introducido en:** § 2

---

## Sección 3 — Hablar con la GPU

### Vulkan loader
**Llano:** la librería centralita que encuentra el driver de tu GPU y le reenvía las llamadas.
**Preciso:** `libvulkan.dylib` (macOS) / `libvulkan.so` (Linux); la carga `ash::Entry::load()`.
**Introducido en:** § 3

### ICD
**Significa:** Installable Client Driver
**Llano:** un driver de Vulkan que el loader puede encontrar mediante un archivo JSON — el tuyo es MoltenVK.
**Introducido en:** § 3

### VkInstance
**Llano:** la conexión de tu programa con Vulkan; se crea la primera y se destruye la última.
**Introducido en:** § 3

### extension
**Llano:** una función opcional de Vulkan que tienes que pedir por su nombre.
**Preciso:** extensiones de instancia (al crear la instancia) o de dispositivo (al crear el
dispositivo). `KHR` = oficial de Khronos, `EXT` = de varios fabricantes.
**Introducido en:** § 3

### layer
**Llano:** código insertado entre tu programa y el driver que ve todas las llamadas.
**Introducido en:** § 3

### validation layer
**Llano:** el inspector que comprueba cada llamada a Vulkan contra las reglas e informa de los errores.
**Preciso:** `VK_LAYER_KHRONOS_validation`; actívala solo en builds de depuración.
**Introducido en:** § 3

### debug messenger
**Llano:** el objeto que pasa los mensajes de validación a una función que escribiste tú, para que puedas imprimirlos.
**Preciso:** `VkDebugUtilsMessengerEXT`, de `VK_EXT_debug_utils`.
**Introducido en:** § 3

### VK_KHR_portability_enumeration
**Significa:** Khronos — portabilidad — enumeración (listado)
**Llano:** el permiso expreso que hace que el loader liste drivers "casi completos" como MoltenVK.
**Preciso:** extensión de instancia; también requiere el flag de instancia `ENUMERATE_PORTABILITY_KHR`.
**Introducido en:** § 3

### VK_KHR_portability_subset
**Llano:** el reconocimiento, en el lado del dispositivo, de que el driver implementa un subconjunto de Vulkan.
**Preciso:** extensión de dispositivo; hay que activarla si el dispositivo la lista.
**Introducido en:** § 3

### physical device
**Llano:** una GPU real de la máquina; las listas y eliges una.
**Preciso:** `VkPhysicalDevice`; no la creas ni la destruyes tú.
**Introducido en:** § 3

### queue family
**Llano:** un grupo de bandejas de entrada de la GPU con las mismas capacidades (dibujar, calcular, copiar), identificado por un índice.
**Introducido en:** § 3

### logical device
**Llano:** tu sesión con la GPU elegida; casi todo se crea a partir de él.
**Preciso:** `VkDevice`; destruye antes todo lo creado a partir de él.
**Introducido en:** § 3

### handle
**Llano:** un número opaco que nombra un objeto de Vulkan — como un descriptor de archivo. Copiarlo no copia el objeto.
**Introducido en:** § 3

### CreateInfo
**Llano:** el struct que rellenas para describir un objeto antes de crearlo.
**Preciso:** `Vk*CreateInfo`, que empieza por `sType` (tipo de estructura) y `pNext` (cadena de extensiones).
**Introducido en:** § 3

### VkResult
**Llano:** el código de éxito/error que devuelve casi toda función de Vulkan.
**Preciso:** en `ash`, aparece como `Result<T, vk::Result>`.
**Introducido en:** § 3

### ash
**Llano:** el crate de Rust que expone 1:1 las funciones en C de Vulkan. Un juego de palabras: lo que queda tras el fuego de Vulkan (ceniza).
**Preciso:** `ash::Entry` (loader), `ash::Instance` / `ash::Device` (tablas de funciones); las llamadas son `unsafe`.
**Introducido en:** § 3

---

## Sección 4 — Llevar imágenes a la pantalla

### surface
**Llano:** el adaptador de Vulkan a tu ventana — "un sitio al que puedo presentar imágenes".
**Preciso:** `VkSurfaceKHR`; se crea a partir de la instancia y la ventana; la ventana debe vivir más que ella.
**Introducido en:** § 4

### VK_KHR_surface
**Llano:** la extensión de instancia que define las surfaces en general.
**Introducido en:** § 4

### VK_EXT_metal_surface
**Llano:** la extensión de instancia propia de macOS que crea una surface a partir de la capa Metal de una ventana.
**Introducido en:** § 4

### raw-window-handle
**Llano:** un crate diminuto que define un formato común de "aquí tienes un handle de ventana" entre librerías de ventanas y de gráficos.
**Introducido en:** § 4

### ash-window
**Llano:** un pequeño crate que convierte un handle de ventana en crudo en la surface de Vulkan adecuada y lista las extensiones necesarias.
**Introducido en:** § 4

### present
**Llano:** entregar una imagen terminada para que se muestre en pantalla.
**Introducido en:** § 4

### VK_KHR_swapchain
**Llano:** la extensión de dispositivo que proporciona swapchains.
**Introducido en:** § 4

### surface capabilities
**Llano:** lo que permite la surface: límites de número de imágenes, tamaño actual, límites de tamaño, transformaciones, modos de alpha.
**Introducido en:** § 4

### surface format
**Llano:** la disposición de píxeles y el espacio de color de las imágenes de la swapchain, p. ej. `B8G8R8A8_SRGB`.
**Introducido en:** § 4

### sRGB
**Significa:** standard Red Green Blue
**Llano:** una codificación de color que dedica más precisión a los tonos oscuros; los formatos `_SRGB` convierten automáticamente al escribir.
**Introducido en:** § 4

### present mode
**Llano:** la regla de cuándo una imagen terminada sustituye a la que está en pantalla.
**Introducido en:** § 4

### FIFO
**Significa:** First In, First Out
**Llano:** vsync: se muestra una imagen de la cola por cada refresco de pantalla. Siempre soportado.
**Introducido en:** § 4

### MAILBOX
**Llano:** como FIFO, pero una imagen terminada más nueva sustituye a la que espera. Opcional.
**Introducido en:** § 4

### extent
**Llano:** un tamaño 2D en píxeles (`ancho × alto`). Siempre píxeles físicos.
**Introducido en:** § 4

### VkImage
**Llano:** un bloque de memoria de la GPU con píxeles, más su descripción. Las imágenes de la swapchain son de la swapchain.
**Introducido en:** § 4

### image view
**Llano:** una descripción de cómo leer una imagen (formato, qué parte); el renderizado siempre pasa por una view.
**Preciso:** `VkImageView`; creas y destruyes una por imagen de la swapchain.
**Introducido en:** § 4

### ERROR_OUT_OF_DATE_KHR
**Llano:** "la swapchain ya no encaja con la ventana" — reconstrúyela. Su hermano: `SUBOPTIMAL_KHR`.
**Introducido en:** § 4

### composite alpha
**Llano:** cómo debe tratar el sistema operativo la transparencia de tu imagen al poner tu ventana sobre el resto de la pantalla. `OPAQUE` = ignorarla, la ventana es sólida.
**Preciso:** `VkCompositeAlphaFlagBitsKHR`, elegido al crear la swapchain a partir de `supported_composite_alpha` en las surface capabilities.
**Introducido en:** § 4

---

## Sección 5 — Darle trabajo a la GPU, y esperar

### command pool
**Llano:** la memoria de la que se recortan los command buffers; uno por queue family.
**Preciso:** `VkCommandPool`; `RESET_COMMAND_BUFFER` te deja reiniciar los buffers de uno en uno. Destruir el pool libera sus buffers.
**Introducido en:** § 5

### recording
**Llano:** escribir instrucciones en un command buffer entre `begin` y `end`. Todavía no se ejecuta nada en la GPU.
**Preciso:** toda llamada `cmd_*` (`cmd_clear_color_image`, `cmd_pipeline_barrier`, …).
**Introducido en:** § 5

### vkQueueSubmit
**Llano:** entregar command buffers a una cola; vuelve enseguida, la GPU los ejecuta después.
**Preciso:** también nombra los semáforos a esperar (y en qué etapa), los semáforos a señalar y una fence a señalar al terminar.
**Introducido en:** § 5

### fence
**Llano:** una señal de la GPU a la CPU: "ese lote de trabajo ha terminado". **La GPU señala, la CPU espera.**
**Preciso:** `VkFence`; `wait_for_fences` / `reset_fences`. Crea las fences de fotograma `SIGNALED` o el primer fotograma espera para siempre.
**Introducido en:** § 5

### semaphore
**Llano:** una señal entre trabajos de la GPU (o entre la GPU y la pantalla): "ya puedes seguir". **La GPU señala, la GPU/la presentación espera** — la CPU nunca lo espera. Se llama así por las señales de semáforo ferroviarias.
**Preciso:** `VkSemaphore` (binario). scop usa `image_free` (por fotograma en vuelo) y `render_done` (por imagen de la swapchain).
**Introducido en:** § 5

### acquire
**Llano:** preguntar a la swapchain en qué imagen dibujar a continuación; recibes el índice enseguida, y se levanta un semáforo cuando la imagen está de verdad fuera de pantalla.
**Preciso:** `vkAcquireNextImageKHR`; puede devolver `ERROR_OUT_OF_DATE_KHR` o informar de suboptimal.
**Introducido en:** § 5

### frames in flight
**Llano:** cuántos fotogramas puede adelantarse la CPU a la GPU. Con 2 la CPU graba el siguiente fotograma mientras la GPU dibuja el actual.
**Preciso:** cada slot de fotograma tiene su propio command buffer, fence y semáforo `image_free`.
**Introducido en:** § 5

### image layout
**Llano:** cómo están organizados los píxeles de una imagen en la memoria de la GPU para un trabajo concreto (que la escriban, dibujar en ella, mostrarla).
**Preciso:** `UNDEFINED`, `TRANSFER_DST_OPTIMAL`, `COLOR_ATTACHMENT_OPTIMAL`, `PRESENT_SRC_KHR`, …
**Introducido en:** § 5

### layout transition
**Llano:** cambiar una imagen de un layout a otro, como un comando grabado de la GPU.
**Preciso:** se hace con una image memory barrier dentro de `cmd_pipeline_barrier`.
**Introducido en:** § 5

### pipeline barrier
**Llano:** "todo lo de este tipo antes de aquí debe terminar antes de que empiece cualquier cosa de aquel tipo después de aquí" — más, opcionalmente, un cambio de layout.
**Preciso:** `vkCmdPipelineBarrier` con etapas y access masks de origen/destino.
**Introducido en:** § 5

### pipeline stage
**Llano:** un paso de la cadena de producción de la GPU; las barreras y las esperas nombran etapas para que solo espere el trabajo que debe esperar.
**Preciso:** `TRANSFER`, `COLOR_ATTACHMENT_OUTPUT`, `TOP_OF_PIPE`, `BOTTOM_OF_PIPE`, …
**Introducido en:** § 5

### access mask
**Llano:** qué tipo de acceso a memoria debe terminar y hacerse visible. Etapa = *cuándo*, acceso = *qué*.
**Preciso:** `TRANSFER_WRITE`, `COLOR_ATTACHMENT_WRITE`, `SHADER_READ`, …; le dice a la GPU qué cachés volcar.
**Introducido en:** § 5

### vkCmdClearColorImage
**Llano:** llenar una imagen entera de un color; sin shaders.
**Preciso:** un comando de transferencia; la imagen debe estar en `TRANSFER_DST_OPTIMAL` y tener el uso `TRANSFER_DST`.
**Introducido en:** § 5

### TRANSFER_DST
**Llano:** "transfer destination" (destino de transferencia) — un flag de uso de imagen que permite copias y clears hacia ella.
**Preciso:** `vk::ImageUsageFlags::TRANSFER_DST`; hay que pedirlo al crear la swapchain y que la surface lo soporte.
**Introducido en:** § 5

### device_wait_idle
**Llano:** esperar hasta que la GPU lo haya terminado todo. Para apagar, nunca en cada fotograma.
**Preciso:** `vkDeviceWaitIdle`.
**Introducido en:** § 5

### RedrawRequested
**Llano:** el evento de `winit` "dibuja esta ventana ahora"; llamar a `window.request_redraw()` tras cada fotograma crea un bucle continuo.
**Preciso:** `WindowEvent::RedrawRequested`.
**Introducido en:** § 5

---

## Sección 6 — Programas que corren en la GPU

### shader module
**Llano:** el handle de Vulkan para un bloque de código de shader compilado.
**Preciso:** `VkShaderModule`, creado a partir de palabras SPIR-V; se puede destruir en cuanto está construido el pipeline.
**Introducido en:** § 6

### entry point
**Llano:** la función por la que empieza un shader — como `main` en C.
**Preciso:** se nombra al construir el pipeline (`c"main"`).
**Introducido en:** § 6

### build script
**Llano:** `build.rs` — un pequeño programa en Rust que Cargo ejecuta antes de compilar el crate. El nuestro compila los shaders.
**Preciso:** imprime líneas `cargo:` para hablar con Cargo, p. ej. `cargo:rerun-if-changed=shaders`; escribe sus resultados en `OUT_DIR`.
**Introducido en:** § 6

### glslc
**Significa:** GLSL compiler (del proyecto `shaderc` de Google).
**Llano:** convierte texto GLSL en bytes SPIR-V. Una herramienta de build, no una librería enlazada en scop.
**Introducido en:** § 6

### include_bytes!
**Llano:** una macro de Rust que copia los bytes de un archivo dentro del programa en tiempo de compilación.
**Preciso:** da `&'static [u8]`; con `concat!(env!("OUT_DIR"), "/x.spv")` incrusta la salida del build script.
**Introducido en:** § 6

### SPIR-V magic number
**Llano:** los 4 primeros bytes con los que empieza todo archivo SPIR-V, `0x07230203` — una comprobación rápida de "¿esto es SPIR-V de verdad?".
**Preciso:** SPIR-V es una lista de palabras little-endian de 32 bits, así que el tamaño debe ser múltiplo de 4.
**Introducido en:** § 6

### graphics pipeline
**Llano:** toda la cadena de producción de dibujo — tus shaders más todos los interruptores fijos — construida una vez y congelada.
**Preciso:** `VkPipeline` de `create_graphics_pipelines`. Cambiar un ajuste no dinámico significa un pipeline nuevo.
**Introducido en:** § 6

### fixed-function state
**Llano:** los interruptores de las etapas del pipeline que no programas (ensamblado, rasterización, mezcla).
**Introducido en:** § 6

### input assembly
**Llano:** cómo se agrupa el flujo de esquinas en formas.
**Preciso:** topología `TRIANGLE_LIST` = cada 3 vértices forman un triángulo.
**Introducido en:** § 6

### viewport
**Llano:** el rectángulo de la imagen sobre el que se estira la caja −1..+1 de NDC.
**Preciso:** normalmente el extent entero de la swapchain; dynamic state en scop.
**Introducido en:** § 6

### scissor
**Llano:** un rectángulo fuera del cual se tiran los píxeles. El viewport aplasta, el scissor recorta.
**Introducido en:** § 6

### dynamic state
**Llano:** ajustes del pipeline que se dejan fuera del pipeline congelado y se ponen mientras se graba.
**Preciso:** scop: `VIEWPORT`, `SCISSOR` (así redimensionar no necesita un pipeline nuevo).
**Introducido en:** § 6

### cull mode
**Llano:** si se saltan los triángulos que miran en dirección contraria a la cámara.
**Preciso:** `NONE`, `BACK`, `FRONT`; qué lado es "delante" lo decide front face.
**Introducido en:** § 6

### front face
**Llano:** qué orden de esquinas (horario o antihorario en pantalla) cuenta como mirando hacia ti.
**Introducido en:** § 6

### colour write mask
**Llano:** qué canales de color (R, G, B, A) puede escribir el pipeline. Por defecto ninguno — olvídala y no aparece nada.
**Introducido en:** § 6

### pipeline layout
**Llano:** la lista de entradas extra (uniforms, texturas) que pueden leer los shaders. Vacía por ahora, pero obligatoria.
**Preciso:** `VkPipelineLayout`.
**Introducido en:** § 6

### dynamic rendering
**Llano:** empezar una pasada de dibujo nombrando las image views en las que se dibuja, directamente en el command buffer.
**Preciso:** `cmd_begin_rendering` / `cmd_end_rendering`; sustituye a los objetos render pass y framebuffer.
**Introducido en:** § 6

### VK_KHR_dynamic_rendering
**Llano:** la extensión de dispositivo que da dynamic rendering en Vulkan 1.2 (incluido de serie desde 1.3).
**Preciso:** necesita la extensión **y** la feature `dynamicRendering` activadas al crear el dispositivo; las funciones vienen de `ash::khr::dynamic_rendering::Device`.
**Introducido en:** § 6

### load op / store op
**Llano:** qué le pasa a la imagen al principio (limpiar / conservar / me da igual) y al final (conservar / me da igual) del dibujo.
**Preciso:** `AttachmentLoadOp::CLEAR`, `AttachmentStoreOp::STORE` en scop.
**Introducido en:** § 6

### gl_VertexIndex
**Llano:** entrada integrada del vertex shader: para qué esquina es esta ejecución (0, 1, 2…).
**Introducido en:** § 6

### gl_Position
**Llano:** salida integrada del vertex shader: dónde cae la esquina, como (x, y, z, w).
**Introducido en:** § 6

### location
**Llano:** un hueco numerado que une una salida del vertex shader con una entrada del fragment shader. Los números deben coincidir; los nombres dan igual.
**Preciso:** `layout(location = N)`.
**Introducido en:** § 6

### interpolation
**Llano:** mezclar valores por esquina a lo largo de un triángulo para cada píxel, ponderados por la distancia a cada esquina. Lo hace el rasterizador.
**Introducido en:** § 6

---

## Sección 7 — Enviar datos a la GPU

### mesh
**Llano:** una forma hecha de esquinas (vértices) y triángulos (índices a las esquinas) — una red de triángulos sobre la superficie. Todo modelo que muestra scop es una.
**Preciso:** `MeshData` (CPU: `Vec<Vertex>` + `Vec<u32>`) → `Mesh` (GPU: vertex buffer + index buffer + número de índices).
**Introducido en:** § 7

### buffer
**Llano:** una descripción de "N bytes que la GPU puede usar para estos fines". No tiene memoria hasta que le enlazas alguna.
**Preciso:** `VkBuffer`; destrúyelo antes de liberar su memoria.
**Introducido en:** § 7

### buffer usage
**Llano:** para qué puede usarse un buffer, declarado al crearlo.
**Preciso:** `VERTEX_BUFFER`, `INDEX_BUFFER`, `TRANSFER_SRC`, `TRANSFER_DST`, `UNIFORM_BUFFER`, …
**Introducido en:** § 7

### device memory
**Llano:** un bloque real de memoria que reservas, y al que luego enganchas buffers (o imágenes).
**Preciso:** `VkDeviceMemory`, de `allocate_memory`; se libera con `free_memory`.
**Introducido en:** § 7

### memory requirements
**Llano:** lo que un buffer te dice que necesita: tamaño, alineación y en qué tipos de memoria puede vivir.
**Preciso:** `memory_type_bits`: bit *i* activado = tipo de memoria *i* permitido. Reserva `size`, no el tamaño de tus datos.
**Introducido en:** § 7

### memory type
**Llano:** un tipo de memoria de la GPU, descrito por flags de propiedades; eliges uno por índice.
**Introducido en:** § 7

### memory heap
**Llano:** una bolsa física de memoria (p. ej. la VRAM de la tarjeta gráfica); los tipos de memoria son formas de usar un heap.
**Introducido en:** § 7

### DEVICE_LOCAL
**Llano:** memoria rápida para la GPU — el almacén.
**Introducido en:** § 7

### HOST_VISIBLE
**Llano:** memoria que la CPU ("host") puede mapear y escribir — el muelle de carga.
**Introducido en:** § 7

### HOST_COHERENT
**Llano:** las escrituras de la CPU en memoria mapeada se hacen visibles para la GPU automáticamente, sin flush manual.
**Introducido en:** § 7

### map
**Llano:** obtener un puntero de la CPU a memoria host-visible, escribir a través de él, y luego deshacer el mapeo.
**Preciso:** `map_memory` / `unmap_memory`.
**Introducido en:** § 7

### bind (memory)
**Llano:** enganchar un buffer a (parte de) una reserva de memoria. Una vez por buffer, para siempre.
**Preciso:** `bind_buffer_memory(buffer, memory, offset)`.
**Introducido en:** § 7

### staging buffer
**Llano:** un buffer temporal que la CPU puede escribir, usado solo para llevar datos a un buffer local de la GPU con una copia.
**Introducido en:** § 7

### cmd_copy_buffer
**Llano:** comando de transferencia grabado: copiar bytes de un buffer a otro.
**Preciso:** el origen necesita `TRANSFER_SRC`, el destino `TRANSFER_DST`.
**Introducido en:** § 7

### vertex buffer
**Llano:** un buffer con los datos de las esquinas (posición, color, …).
**Introducido en:** § 7

### index buffer
**Llano:** un buffer de números de esquina; cada 3 forman un triángulo, así que las esquinas compartidas se guardan una vez.
**Introducido en:** § 7

### index type
**Llano:** el tamaño de cada índice: `UINT16` (≤ 65 535 esquinas) o `UINT32`. scop usa `UINT32`.
**Introducido en:** § 7

### stride
**Llano:** bytes desde el inicio de un vértice hasta el inicio del siguiente.
**Introducido en:** § 7

### vertex binding
**Llano:** "el hueco de binding N lee un buffer cuyos vértices están separados `stride` bytes".
**Preciso:** `VertexInputBindingDescription`, input rate `VERTEX`.
**Introducido en:** § 7

### vertex attribute
**Llano:** "la `location` N del shader está en el byte `offset` de cada vértice, en este formato".
**Preciso:** `VertexInputAttributeDescription`.
**Introducido en:** § 7

### attribute format
**Llano:** cuántos números y de qué tipo es un atributo. `R32G32_SFLOAT` = dos floats de 32 bits (`vec2`); aquí R/G/B solo significan 1ª/2ª/3ª componente.
**Introducido en:** § 7

### offset_of!
**Llano:** macro de Rust: el desplazamiento en bytes de un campo dentro de un struct.
**Introducido en:** § 7

### draw_indexed
**Llano:** dibujar recorriendo el index buffer en vez de los vértices en orden.
**Preciso:** `cmd_draw_indexed(index_count, instance_count, first_index, vertex_offset, first_instance)`.
**Introducido en:** § 7

---

## Sección 8 — Separar el engine de la app

### public API
**Llano:** todo lo que un crate hace `pub` y accesible desde fuera — la superficie acordada que puede usar otro código.
**Introducido en:** § 8

### crate boundary
**Llano:** la línea entre dos crates; Rust solo deja que uno use lo que el otro marca como `pub`.
**Introducido en:** § 8

### coupling
**Llano:** cuánto depende una pieza de código de los detalles de otra. Acoplamiento débil = las tripas pueden cambiar sin romper a quien lo usa.
**Introducido en:** § 8

### inversion of control
**Llano:** el framework es dueño del bucle principal y llama a tu código en los momentos adecuados — "no nos llames, ya te llamamos nosotros".
**Introducido en:** § 8

### App trait
**Llano:** el contrato que implementa una app para que el engine pueda llamarla: `setup`, `update`, `draw`.
**Introducido en:** § 8

### default method
**Llano:** un método de trait con cuerpo; quien lo implementa puede sobrescribirlo o dejarlo.
**Introducido en:** § 8

### generic function
**Llano:** una función que sirve para cualquier tipo que cumpla un requisito: `fn run<A: App>(app: A)`. Se especializa en compilación, sin coste en ejecución.
**Introducido en:** § 8

### newtype
**Llano:** un struct que envuelve un valor para crear un tipo distinto: `struct MeshId(usize)`.
**Introducido en:** § 8

### typed handle
**Llano:** un id newtype que el engine da en vez del objeto; el engine conserva el ownership.
**Preciso:** `MeshId`, `PipelineId` — índices a los `Vec` del engine, con un número interno privado.
**Introducido en:** § 8

### draw list
**Llano:** la lista de peticiones por fotograma de la app ("limpia a X; dibuja la mesh M con el pipeline P"), que graba el engine.
**Introducido en:** § 8

### input abstraction
**Llano:** el propio tipo `Key` del engine, traducido del de `winit`, para que las apps nunca dependan de `winit`.
**Introducido en:** § 8

### delta time
**Llano:** segundos desde el fotograma anterior; multiplica el movimiento por él para que la velocidad no dependa de la tasa de fotogramas.
**Introducido en:** § 8

### re-export
**Llano:** `pub use path::Item;` — hace que un elemento esté disponible en una ruta pública más corta.
**Introducido en:** § 8

---

## Sección 9 — Las matemáticas 3D justas

### vector
**Llano:** tres números (x, y, z): un lugar o un paso. Tiene longitud y dirección.
**Introducido en:** § 9

### point vs direction
**Llano:** los mismos tres números pueden significar un lugar (lo mueven las traslaciones) o un paso/flecha (no se mueve). Se distinguen por `w`.
**Introducido en:** § 9

### dot product
**Llano:** `a·b = ax·bx + ay·by + az·bz`; para vectores de longitud 1, el coseno del ángulo entre ellos (1 misma dirección, 0 perpendiculares, −1 opuestos).
**Introducido en:** § 9

### cross product
**Llano:** `a × b`: un vector en ángulo recto con los dos — p. ej. hacia dónde mira una cara, a partir de dos de sus aristas.
**Introducido en:** § 9

### normalise
**Llano:** escalar un vector a longitud 1, conservando su dirección. Protégete contra la longitud 0.
**Introducido en:** § 9

### matrix
**Llano:** una cuadrícula de 4×4 números que aplica una transformación; multiplicar dos las combina.
**Preciso:** `Mat4`, guardada en column-major.
**Introducido en:** § 9

### transform
**Llano:** un cambio aplicado a todos los puntos — mover, girar, redimensionar, proyectar.
**Introducido en:** § 9

### homogeneous coordinates
**Llano:** escribir los vectores 3D con un 4º número `w`: 1 para puntos, 0 para direcciones. Permite que una matriz de 4×4 también traslade, y solo a los puntos.
**Introducido en:** § 9

### translation matrix
**Llano:** mueve los puntos por (tx, ty, tz); los números van en la 4ª columna.
**Introducido en:** § 9

### rotation matrix
**Llano:** gira los puntos alrededor del eje x, y o z que pasa por el origen, un ángulo en radianes.
**Introducido en:** § 9

### scale matrix
**Llano:** estira o encoge a lo largo de cada eje.
**Introducido en:** § 9

### composition
**Llano:** combinar transformaciones multiplicando matrices. Con vectores columna, `A × B` hace primero B — se lee de derecha a izquierda. El orden cambia el resultado.
**Introducido en:** § 9

### model matrix
**Llano:** coloca un modelo en el mundo: centrarlo, girarlo, moverlo.
**Introducido en:** § 9

### view matrix
**Llano:** coloca el mundo delante de la cámara (scop: empujarlo lejos a lo largo de −z).
**Introducido en:** § 9

### projection matrix
**Llano:** prepara la escena para la pantalla con perspectiva; la versión de Vulkan además invierte y y lleva la profundidad a 0..1.
**Preciso:** la cadena completa es `P × V × M`.
**Introducido en:** § 9

### perspective divide
**Llano:** la GPU divide x, y, z entre `w` tras el vertex shader; con `w` = distancia, lo lejano se encoge.
**Introducido en:** § 9

### field of view
**Llano:** cuánto de ancho ve la cámara, como ángulo. scop usa el vertical.
**Introducido en:** § 9

### aspect ratio
**Llano:** ancho ÷ alto de la imagen; la proyección lo usa para que nada se vea estirado.
**Introducido en:** § 9

### near and far planes
**Llano:** las distancias más cercana y más lejana que se dibujan; profundidad 0 en near, 1 en far.
**Introducido en:** § 9

### column-major
**Llano:** guardar los 16 números de una matriz columna a columna — como lee GLSL los `mat4`.
**Introducido en:** § 9

### radians
**Llano:** unidad de ángulo: una vuelta completa = 2π ≈ 6.28. `grados × π / 180`.
**Introducido en:** § 9

### operator overloading
**Llano:** implementar los traits de `std::ops` (`Add`, `Mul`…) para que tus tipos funcionen con `+`, `*`.
**Introducido en:** § 9

### epsilon comparison
**Llano:** considerar iguales dos floats si su diferencia está por debajo de un umbral diminuto, por el redondeo.
**Introducido en:** § 9

## Sección 10 — Pasar datos por fotograma a los shaders

### uniform
**Llano:** una entrada de shader con el mismo valor para todas las esquinas y píxeles de un draw.
**Preciso:** bloque `uniform` de GLSL; lo alimenta un descriptor (buffer) o los push constants.
Lo opuesto a las entradas por esquina del vertex buffer.
**Introducido en:** § 10

### per-frame vs per-draw data
**Llano:** por fotograma = cambia como mucho una vez por fotograma, lo comparten todos los draws (la cámara);
por draw = puede ser distinto entre draws de un mismo fotograma (la matriz de modelo de cada objeto).
**Introducido en:** § 10

### push constant
**Llano:** un pequeño bloque de bytes grabado directamente en el command buffer para los draws siguientes — el "pósit".
**Preciso:** `cmd_push_constants`; al menos 128 bytes garantizados (`maxPushConstantsSize`).
*Push* = empujado dentro de la grabación; *constant* = fijo durante el draw que lo lee.
**Introducido en:** § 10

### push constant range
**Llano:** la declaración "los shaders leen bytes de push constant de aquí a aquí, en esta etapa".
**Preciso:** `VkPushConstantRange { stage_flags, offset, size }`, parte del pipeline layout.
**Introducido en:** § 10

### stage flags
**Llano:** qué etapas de shader pueden ver una entrada.
**Preciso:** `VkShaderStageFlags` — `VERTEX`, `FRAGMENT`, … Etapa equivocada = error de validación.
**Introducido en:** § 10

### pipeline layout
**Llano:** la lista de entradas de fuera del pipeline — como la firma de función de sus parámetros extra.
**Preciso:** `VkPipelineLayout`: descriptor set layouts + push constant ranges. Los shaders solo
pueden leer lo que declara. Los pipelines que comparten layout conservan los sets enlazados entre `cmd_bind_pipeline`.
**Introducido en:** § 10 (vacío desde § 6)

### uniform buffer
**Llano:** un buffer que el shader lee como datos uniform — el "tablón de anuncios".
**Preciso:** `VkBuffer` con uso `UNIFORM_BUFFER`. Nombre en OpenGL: UBO, Uniform Buffer Object.
**Introducido en:** § 10

### persistently mapped
**Llano:** mapear el buffer una vez y guardar el puntero de la CPU durante toda su vida.
**Preciso:** sin map/unmap por fotograma; funciona sin flushes en memoria `HOST_COHERENT`.
**Introducido en:** § 10

### descriptor
**Llano:** un registro diminuto que le dice a un shader dónde está un recurso — apunta a los datos, no son los datos.
**Preciso:** p. ej. `VkDescriptorBufferInfo { buffer, offset, range }` escrito en un set.
**Introducido en:** § 10

### descriptor set layout
**Llano:** la forma de un descriptor set, como la declaración de un struct.
**Preciso:** `VkDescriptorSetLayout`: por binding — tipo, cantidad, stage flags.
**Introducido en:** § 10

### descriptor pool
**Llano:** de donde se reservan los descriptor sets, dimensionado por adelantado.
**Preciso:** `VkDescriptorPool`; destruirlo libera todos los sets reservados de él (como el command pool, § 5).
**Introducido en:** § 10

### descriptor set
**Llano:** un grupo de descriptors enganchados en una llamada — una instancia de un set layout.
**Preciso:** `VkDescriptorSet`, reservado de un pool, rellenado con descriptor writes.
**Introducido en:** § 10

### set and binding numbers
**Llano:** la dirección de dos niveles de un descriptor para el shader.
**Preciso:** GLSL `layout(set = S, binding = B)`: hueco de set S, entrada B dentro de ese set.
**Introducido en:** § 10

### descriptor write
**Llano:** hacer que una entrada de un set apunte a un recurso concreto. Se hace una vez; después cambias el contenido, no el puntero.
**Preciso:** `VkWriteDescriptorSet` + `update_descriptor_sets`.
**Introducido en:** § 10

### binding a descriptor set
**Llano:** grabar en el command buffer "a partir de ahora, el hueco de set 0 es este set".
**Preciso:** `cmd_bind_descriptor_sets(cmd, GRAPHICS, layout, first_set, sets, dynamic_offsets)`.
**Introducido en:** § 10

### std140
**Llano:** la regla de GLSL para disponer los bytes de un bloque uniform; un `mat4` son 64 bytes, columna a columna.
**Preciso:** disposición **st**an**d**ard de GLSL **1.40**. Trampa: un `vec3` ocupa 16 bytes. scop solo envía `mat4`s.
**Introducido en:** § 10

### view-projection matrix
**Llano:** `projection × view` combinadas una vez por fotograma en la CPU, para que el shader multiplique por menos matrices.
**Introducido en:** § 10

---

## Chuleta de sincronización — fence vs semáforo vs barrera

La misma idea ("espera a que eso termine"), distinto **quien espera**. Pregúntate *¿quién espera a quién?*

| Nombre | Quién espera | A qué | Ejemplo en scop |
|---|---|---|---|
| **Fence** | la **CPU** | a que la GPU termine un submit | esperar antes de reutilizar un slot de fotograma (§ 5); esperar antes de destruir el staging buffer (§ 7) |
| **Semáforo** | la **GPU** | a otra operación de la GPU (entre submits / present) | no dibujar hasta que la imagen esté libre (`image_free`); no mostrarla hasta que termine el dibujo (`render_done`) (§ 5) |
| **Barrera** | los **comandos posteriores** del *mismo* command buffer | a los comandos anteriores en él | copia terminada → pueden empezar las lecturas de vértices (§ 7); cambios de layout de imagen (§ 5) |

- **Fence:** una puerta que la CPU no puede cruzar hasta que la GPU la abre.
- **Semáforo:** una señal ferroviaria entre trabajos de la GPU. Solo los semáforos tienen una *etapa de espera*.
- **Barrera:** una línea dentro de una lista de tareas que los pasos posteriores no pueden cruzar hasta que terminen los anteriores.

## Sección 11 — Qué está delante de qué

### occlusion
**Llano:** una cosa que tapa a otra. Oclusión correcta = lo cercano tapa lo lejano.
**Introducido en:** § 11

### painter's algorithm
**Llano:** dibujar de lejos a cerca para que lo cercano pinte encima de lo lejano.
**Preciso:** hay que reordenar cada vez que algo se mueve, y falla con triángulos que se cruzan. No se usa en scop.
**Introducido en:** § 11

### depth test
**Llano:** por píxel: "¿este fragmento nuevo está más cerca que lo que ya hay?". Si no, se tira.
**Introducido en:** § 11

### depth buffer
**Llano:** un número por píxel — la profundidad de lo que haya dibujado ahí en ese momento. También: z-buffer.
**Introducido en:** § 11

### depth value
**Llano:** el número guardado: `z / w` tras la división de perspectiva; 0 = plano cercano, 1 = plano lejano.
**Introducido en:** § 11

### compare op
**Llano:** la regla de "más cerca". scop: `LESS` (gana la profundidad menor).
**Preciso:** `VkCompareOp`, "operación de comparación", puesta en el depth/stencil state del pipeline.
**Introducido en:** § 11

### depth write
**Llano:** tras pasar un fragmento, guardar su profundidad. Un interruptor distinto del test.
**Introducido en:** § 11

### depth clear value
**Llano:** a qué se reinicia la profundidad de cada píxel cada fotograma: 1.0, el plano lejano.
**Introducido en:** § 11

### depth image
**Llano:** el depth buffer como objeto de Vulkan — un `VkImage` del tamaño de la swapchain en memoria `DEVICE_LOCAL`, más una view.
**Introducido en:** § 11

### depth format
**Llano:** cómo se guarda cada número de profundidad. scop prefiere `D32_SFLOAT` — **D**epth, **32** bits, **S**igned **FLOAT**.
**Introducido en:** § 11

### format features
**Llano:** lo que puede hacer una GPU con un formato.
**Preciso:** `get_physical_device_format_properties`; un formato de profundidad necesita `DEPTH_STENCIL_ATTACHMENT` en sus features con tiling óptimo.
**Introducido en:** § 11

### image aspect
**Llano:** de qué parte de una imagen habla una view o una barrera: `COLOR` o `DEPTH`.
**Introducido en:** § 11

### depth attachment
**Llano:** la imagen de profundidad enchufada al renderizado — con dynamic rendering, `RenderingInfo::depth_attachment`.
**Introducido en:** § 11

### depth layout
**Llano:** el layout en el que debe estar una imagen de profundidad mientras se usa para el test de profundidad.
**Preciso:** `DEPTH_STENCIL_ATTACHMENT_OPTIMAL`, al que se llega con una barrera por fotograma.
**Introducido en:** § 11

### store op DONT_CARE
**Llano:** "me da igual": tras renderizar, la GPU puede tirar este contenido. Lo correcto para la profundidad — nadie la lee después del fotograma.
**Introducido en:** § 11

### early and late fragment tests
**Llano:** las etapas del pipeline donde ocurre el test de profundidad — antes del fragment shader (temprano, rápido) o después (tardío).
**Introducido en:** § 11

### stencil
**Llano:** un segundo número opcional por píxel para efectos de máscara (contornos, espejos). Sin uso en scop; es la razón de que los layouts digan `DEPTH_STENCIL`.
**Introducido en:** § 11

### z-fighting
**Llano:** dos superficies a casi la misma profundidad parpadean en rayas porque el redondeo elige al ganador. Peor con un plano cercano diminuto.
**Introducido en:** § 11

### back-face culling
**Llano:** saltarse los triángulos que miran en dirección contraria a la cámara. Una optimización, no un sustituto del depth buffer. Desactivado en scop.
**Introducido en:** § 11

### winding order
**Llano:** si las esquinas de un triángulo van en sentido horario o antihorario en pantalla; el culling lo usa para distinguir delante de detrás.
**Introducido en:** § 11

## Sección 12 — Leer archivos .obj

### Wavefront OBJ
**Llano:** el formato de texto `.obj`: una instrucción por línea. Del software de Wavefront Technologies de los años 80.
**Introducido en:** § 12

### vertex line
**Llano:** `v x y z` — la posición de una esquina. Las esquinas se numeran en el orden del archivo, desde 1.
**Introducido en:** § 12

### face line
**Llano:** `f a b c …` — un polígono, que lista sus esquinas por número (3 o más).
**Introducido en:** § 12

### 1-based index
**Llano:** contar desde 1. La esquina `n` del `.obj` es `positions[n − 1]`.
**Introducido en:** § 12

### negative index
**Llano:** `−k` = la k-ésima esquina más reciente definida hasta ahora (−1 = la última). Índice = `len − k`.
**Introducido en:** § 12

### face token
**Llano:** un elemento de una línea de cara: `5`, `5/2`, `5/2/7` o `5//7` — esquina / coordenada de textura / normal. scop usa la primera parte.
**Introducido en:** § 12

### texture coordinate and normal lines
**Llano:** `vt u v` (en qué punto de una imagen está una esquina) y `vn x y z` (hacia dónde mira la superficie). Saltadas en § 12.
**Introducido en:** § 12

### material library
**Llano:** `mtllib archivo.mtl` / `usemtl nombre` — colores en un archivo `.mtl` aparte. Saltado: el subject quiere grises.
**Introducido en:** § 12

### triangulation
**Llano:** cortar un polígono en triángulos, porque la GPU solo dibuja triángulos.
**Introducido en:** § 12

### fan triangulation
**Llano:** fijar la primera esquina y emparejarla con cada arista siguiente: `a b c d e` → `a b c`, `a c d`, `a d e`. Correcto para caras convexas.
**Introducido en:** § 12

### convex and concave
**Llano:** convexo = todas las esquinas apuntan hacia fuera (un abanico siempre funciona); cóncavo = alguna esquina apunta hacia dentro (un abanico puede dibujar fuera de la forma — bonus de § 17).
**Introducido en:** § 12

### bounding box
**Llano:** la caja más pequeña alineada con los ejes que contiene todas las esquinas: mín y máx de cada coordenada.
**Introducido en:** § 12

### model centre
**Llano:** el centro de la bounding box, `(min + max) / 2` — sobre lo que gira el modelo.
**Introducido en:** § 12

### framing
**Llano:** elegir la distancia de la cámara para que quepa el modelo entero: `radius / sin(fov_y / 2)`.
**Introducido en:** § 12

### parse error with line number
**Llano:** un archivo roto da `archivo:17: motivo` y una salida de fallo limpia — nunca un panic.
**Introducido en:** § 12

## Sección 13 — Caras grises y mover el modelo

### flat colour per face
**Llano:** todos los píxeles de una cara del mismo color — las tres esquinas de un triángulo reciben el mismo color, así que no hay nada que mezclar.
**Introducido en:** § 13

### shared corners
**Llano:** cada posición se guarda una vez y la reutilizan todos los triángulos que la tocan (mediante índices). Compacto, pero un valor por esquina.
**Introducido en:** § 13

### duplicated corners
**Llano:** cada triángulo recibe sus propias tres esquinas, así que cada esquina puede llevar un valor para una sola cara (un gris ahora; normales y costuras más adelante).
**Introducido en:** § 13

### face of a triangle
**Llano:** de qué línea de cara del `.obj` salió un triángulo; los dos triángulos de un quad la comparten. `ObjModel::triangle_faces`.
**Introducido en:** § 13

### spread-out greys
**Llano:** `0.25 + 0.5 × fract(i × 0.618…)` — avanza por el rango de grises para que caras consecutivas siempre difieran, sin dejar de ser sutiles (0.25..0.75).
**Introducido en:** § 13

### held vs pressed
**Llano:** `held` = cada fotograma que la tecla está abajo (movimiento); `pressed` = solo el fotograma en que baja (interruptores).
**Introducido en:** § 13

### per-second speed
**Llano:** velocidades en unidades por segundo, multiplicadas por `dt`, para que sean iguales a cualquier tasa de fotogramas.
**Introducido en:** § 13

### model state
**Llano:** posición (x, y, z) y ángulos de giro — los cambian las teclas en `update`, se convierten en matriz en `draw`.
**Introducido en:** § 13

### model matrix recipe
**Llano:** `translation(position) × rotation × translation(−centre)` — centrarlo, girarlo, moverlo (se lee de derecha a izquierda).
**Introducido en:** § 13

### world-space movement
**Llano:** moverse según los ejes de la pantalla esté como esté girado el modelo — la tarjeta de traslación va la más a la izquierda.
**Introducido en:** § 13

### Euler angles
**Llano:** un giro descrito como tres ángulos, uno por eje, aplicados en un orden fijo. Por Leonhard Euler.
**Introducido en:** § 13

### gimbal lock
**Llano:** en algunos ángulos dos ejes de Euler se alinean, así que dos teclas giran el modelo igual. Inofensivo aquí; los cuaterniones lo evitan.
**Introducido en:** § 13

## Sección 14 — Imágenes sobre superficies

### texture
**Llano:** una imagen usada para colorear superficies. En Vulkan: imagen + view + sampler, alcanzados a través de un descriptor.
**Introducido en:** § 14

### texel
**Llano:** un cuadradito de color de una textura — **tex**ture **el**ement (píxel sigue siendo "en pantalla").
**Introducido en:** § 14

### texture coordinates
**Llano:** la posición de una esquina en la imagen: U en horizontal, V en vertical, ambas 0..1. "UVs".
**Introducido en:** § 14

### planar mapping
**Llano:** inventar las UVs proyectando la imagen en línea recta a través del modelo a lo largo de un eje.
**Introducido en:** § 14

### stretching
**Llano:** las caras paralelas a la proyección reciben una franja fina de la imagen emborronada por encima. Bonus de § 18.
**Introducido en:** § 14

### TGA
**Llano:** **T**ruevision **G**raphics **A**dapter ("Targa"): cabecera de 18 bytes + píxeles. Sencillo de cargar a mano.
**Introducido en:** § 14

### image header
**Llano:** los bytes fijos al principio de un archivo que describen el resto: tipo, ancho, alto, bits por píxel, flag de origen.
**Introducido en:** § 14

### BGR order
**Llano:** TGA guarda los píxeles como azul, verde, rojo (alpha) — el cargador los pasa a RGBA.
**Introducido en:** § 14

### image origin
**Llano:** si la primera fila guardada es la de abajo (lo que hace TGA por defecto) o la de arriba (bit 5 del descriptor). Ignorarlo = boca abajo.
**Introducido en:** § 14

### run-length encoding
**Llano:** "los N píxeles siguientes son de este color" guardado una vez. TGA tipo 10.
**Introducido en:** § 14

### sRGB
**Llano:** cómo guardan el brillo los archivos de imagen (más detalle en los oscuros). Las texturas deben crearse `…_SRGB` para que la GPU lo decodifique; mal = desvaído o demasiado oscuro.
**Introducido en:** § 14

### texture image
**Llano:** un `VkImage` con uso `TRANSFER_DST | SAMPLED`, `DEVICE_LOCAL`, tiling `OPTIMAL`.
**Introducido en:** § 14

### layout transition for upload
**Llano:** `UNDEFINED → TRANSFER_DST_OPTIMAL` antes de la copia, `→ SHADER_READ_ONLY_OPTIMAL` después.
**Introducido en:** § 14

### copy buffer to image
**Llano:** `cmd_copy_buffer_to_image`: los bytes de staging a los texels de la imagen.
**Introducido en:** § 14

### sampler
**Llano:** `VkSampler`: las reglas para leer una textura (filtrado, bordes). Separado de la imagen.
**Introducido en:** § 14

### filtering
**Llano:** entre texels: `NEAREST` (el más cercano, pixelado) o `LINEAR` (mezclar vecinos, suave).
**Introducido en:** § 14

### address mode
**Llano:** para UVs fuera de 0..1: `REPEAT` (mosaico) o `CLAMP_TO_EDGE` (estirar el borde).
**Introducido en:** § 14

### combined image sampler
**Llano:** un descriptor que apunta a una image view y a un sampler a la vez. GLSL `sampler2D`.
**Introducido en:** § 14

### descriptor set per texture
**Llano:** cada textura tiene su propio set (set 1), escrito una vez; un draw enlaza el que quiere.
**Introducido en:** § 14

### default white texture
**Llano:** una textura blanca de 1×1 siempre enlazada cuando un draw no pide ninguna — multiplicar por blanco no cambia nada.
**Introducido en:** § 14

### mipmaps
**Llano:** copias ya reducidas de una textura para superficies lejanas, para evitar el centelleo. Saltadas en scop.
**Introducido en:** § 14

## Sección 15 — Fundir el color en la textura

### blend factor
**Llano:** el número 0..1 que dice cuánto del segundo color usar — aquí, cuánta imagen frente a gris.
**Introducido en:** § 15

### linear interpolation
**Llano:** `a + (b − a) × t`, el punto a una fracción `t` del camino de `a` a `b`. "Lerp"; GLSL `mix(a, b, t)`.
**Introducido en:** § 15

### target value
**Llano:** hacia dónde va el blend factor (0 o 1). La tecla invierte el objetivo, no el valor.
**Introducido en:** § 15

### transition duration
**Llano:** cuánto dura un fundido completo (0.6 s); cada fotograma el valor se mueve `dt / duration`, limitado.
**Introducido en:** § 15

### easing
**Llano:** remodelar el progreso 0..1 para que el movimiento empiece y termine con suavidad en vez de bruscamente.
**Introducido en:** § 15

### smoothstep
**Llano:** la curva de suavizado clásica `t² × (3 − 2t)` — pendiente cero en los dos extremos, así que no hay "tirón".
**Introducido en:** § 15

### reversible transition
**Llano:** una que puede cambiar de dirección en cualquier momento sin salto — gratis cuando mueves un valor hacia un objetivo.
**Introducido en:** § 15

### push constant range for two stages
**Llano:** un rango visible para el vertex **y** el fragment shader: modelo (64 bytes) + blend (4 bytes) = 68.
**Introducido en:** § 15

## Sección 16 — Dejar la parte obligatoria perfecta

### swapchain recreation
**Llano:** tras redimensionar, esperar a la GPU, destruir la swapchain vieja y crear una nueva al nuevo tamaño de la ventana (mismo formato y present mode).
**Introducido en:** § 16

### out of date and suboptimal
**Llano:** resultados de acquire/present tras redimensionar: out of date = inutilizable, reconstruir; suboptimal = sigue funcionando pero ya no encaja. scop reconstruye en los dos casos.
**Introducido en:** § 16

### size-dependent objects
**Llano:** lo que se reconstruye con la swapchain: sus imágenes y views, y la imagen de profundidad. No los pipelines (viewport dinámico), la cámara, las meshes ni las texturas.
**Introducido en:** § 16

### dependency check
**Llano:** un script que busca cada herramienta necesaria, informa de lo que falta e instala lo que puede instalar con seguridad.
**Introducido en:** § 16

### Makefile target
**Llano:** una receta con nombre: `make` (deps + build), `make run`, `make clean`, `fclean`, `re`.
**Introducido en:** § 16

### release build
**Llano:** `cargo build --release` — optimizado, y sin la validation layer.
**Introducido en:** § 16

### self-evaluation
**Llano:** comprobar tú mismo cada requisito del subject, con una forma de demostrarlo, antes de la evaluación.
**Introducido en:** § 16

### defence
**Llano:** la conversación de evaluación: el evaluador ejecuta scop, prueba sus propios archivos y te pide que expliques el código y las decisiones.
**Introducido en:** § 16

## Sección 17 — Caras cóncavas y no planas

### reflex corner
**Llano:** una esquina que apunta hacia dentro (ángulo interior de más de 180°). Solo las tienen las caras cóncavas.
**Introducido en:** § 17

### ear
**Llano:** tres esquinas consecutivas cuyo triángulo queda entero dentro del polígono — se puede cortar sin peligro.
**Introducido en:** § 17

### ear clipping
**Llano:** triangular cortando orejas de una en una hasta que queda un triángulo (`n − 2` triángulos).
**Introducido en:** § 17

### polygon normal
**Llano:** la dirección hacia la que mira una cara (una dirección, `w = 0`).
**Introducido en:** § 17

### Newell's method
**Llano:** una normal de polígono calculada a partir de todas las esquinas, así que también es correcta para caras cóncavas y dobladas.
**Introducido en:** § 17

### flattening a polygon
**Llano:** quitar el eje hacia el que más apunta la normal, para obtener un contorno 2D para los tests de oreja.
**Introducido en:** § 17

### point-in-triangle test
**Llano:** un punto 2D está dentro de un triángulo si está del mismo lado de las tres aristas.
**Introducido en:** § 17

### self-intersecting polygon
**Llano:** una cara cuyas aristas se cruzan (una pajarita). El ear clipping puede atascarse; scop hace un abanico con el resto.
**Introducido en:** § 17

### open edge
**Llano:** una arista que usa un solo triángulo — la superficie tiene un borde ahí. El borde de la tapa de la tetera tiene ~100.
**Introducido en:** § 17

### area check
**Llano:** una triangulación correcta de una cara plana cubre exactamente su área; un abanico malo cubre más.
**Introducido en:** § 17

## Sección 18 — Mejor texturizado, modos de vista y zoom

### box mapping
**Llano:** por cara, usar una de tres proyecciones (a lo largo de x, y o z) — la que más tenga de frente la cara. Ninguna cara queda de canto, así que no hay rayas.
**Introducido en:** § 18

### texel density
**Llano:** texels por unidad de superficie. Igual en las dos direcciones (una escala para u y v) = sin estiramiento.
**Introducido en:** § 18

### texture seam
**Llano:** una arista donde la imagen salta porque las caras vecinas usan proyecciones distintas.
**Introducido en:** § 18

### triplanar mapping
**Llano:** mezclar las tres proyecciones por píxel en el shader para esconder las costuras. Más suave, más caro; no se hace aquí.
**Introducido en:** § 18

### object line
**Llano:** `o nombre` (o `g nombre`) en un `.obj`: las caras siguientes pertenecen a un objeto nuevo.
**Introducido en:** § 18

### per-object bounding box
**Llano:** mín/máx de las esquinas de un objeto — el marco al que se ajusta su imagen.
**Introducido en:** § 18

### one draw per object
**Llano:** cada objeto su propia mesh y su `draw_textured`, para que cada uno pueda enlazar su propia imagen.
**Introducido en:** § 18

### polygon mode
**Llano:** cómo se rasterizan los triángulos: `FILL` (sólido), `LINE` (aristas), `POINT` (esquinas).
**Introducido en:** § 18

### fillModeNonSolid
**Llano:** la feature de dispositivo que necesitan los modos `LINE`/`POINT` — hay que activarla al crear el dispositivo, no basta con que esté soportada.
**Introducido en:** § 18

### portability subset
**Llano:** lo que no puede hacer Vulkan-sobre-Metal (MoltenVK). P. ej. `pointPolygons`: en Metal no hay triángulos-como-puntos.
**Introducido en:** § 18

### point list
**Llano:** topología de pipeline `POINT_LIST`: cada vértice se dibuja como un punto. El modo puntos de scop.
**Introducido en:** § 18

### gl_PointSize
**Llano:** lo que escribe el vertex shader para fijar el tamaño de cada punto en píxeles; obligatorio con puntos.
**Introducido en:** § 18

### zoom vs dolly
**Llano:** zoom = cerrar el campo de visión (aspecto más plano); dolly = acercarse (más perspectiva).
**Introducido en:** § 18

### scroll delta
**Llano:** cuánto se movió la rueda del ratón en este fotograma; líneas (ratón) y píxeles (trackpad) convertidos en un solo número.
**Introducido en:** § 18

### frames per second
**Llano:** fotogramas terminados por segundo, promediados en una ventana corta (0.5 s en scop).
**Introducido en:** § 19

### frame time
**Llano:** milisegundos por fotograma (`1000 ÷ FPS`); lo que comparan los programadores, porque se suma.
**Introducido en:** § 19

### vsync cap
**Llano:** la presentación FIFO espera al refresco de la pantalla, así que los FPS no pueden pasar de 60 / 120 / ….
**Introducido en:** § 19

### averaging window
**Llano:** cuánto tiempo se cuentan fotogramas antes de actualizar el número de FPS.
**Introducido en:** § 19

### title throttling
**Llano:** cambiar el título de la ventana solo cuando cambia su texto, no cada fotograma.
**Introducido en:** § 19

### dropped file
**Llano:** el evento `DroppedFile(path)` de winit: se arrastró un archivo a la ventana.
**Introducido en:** § 19

### runtime mesh replacement
**Llano:** crear y destruir meshes después de `setup`, mientras hay fotogramas en vuelo — esperar antes de destruir.
**Introducido en:** § 19

### stale handle
**Llano:** un id de algo ya destruido; usarlo debe ser un error reportado, nunca un crash.
**Introducido en:** § 19

### debug view
**Llano:** colores falsos que muestran un número oculto (qué lado te mira, a qué distancia está…).
**Introducido en:** § 19

### gl_FrontFacing
**Llano:** entrada del fragment shader: true si este triángulo se ve por delante. Siempre true para puntos.
**Introducido en:** § 19

### winding order
**Llano:** si las esquinas de un triángulo van en sentido horario o antihorario en pantalla; decide delante vs detrás.
**Introducido en:** § 19

### inside-out face
**Llano:** una cara listada en sentido horario vista desde fuera, así que su delante apunta hacia dentro del modelo.
**Introducido en:** § 19

### linear depth
**Llano:** distancia a la cámara en la dirección de la vista; tras la tarjeta de perspectiva es `w`.
**Introducido en:** § 19

### push params
**Llano:** cuatro números definidos por la app en el bloque de push junto a la matriz de modelo (scop: fundido, modo de depuración, rango de profundidad).
**Introducido en:** § 19
