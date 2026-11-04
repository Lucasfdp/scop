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
