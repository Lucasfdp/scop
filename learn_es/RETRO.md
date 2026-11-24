# scop — Retrospectiva

## Qué sé hacer ahora que antes no

- **Levantar Vulkan desde cero**: instancia, validation layers, dispositivo, surface, swapchain,
  command buffers, un bucle de fotogramas con dos fotogramas en vuelo — en macOS a través de
  MoltenVK, y en las máquinas Linux del campus.
- **Explicar la sincronización de la GPU según quién espera a quién**: fences (espera la CPU),
  semáforos (espera la GPU), barreras (esperan los comandos posteriores) — y detectar las carreras
  silenciosas (un buffer de cámara compartido, una barrera de profundidad que falta) de las que la
  validación nunca informa.
- **Llevar datos a la GPU de la forma correcta**: staging a memoria device-local para lo que se
  escribe una vez, memoria host-visible mapeada para lo que se escribe cada fotograma; descriptors
  como punteros, push constants como pósits.
- **Escribir y conectar shaders**: GLSL → SPIR-V al compilar, mi propio cargador de SPIR-V,
  pipelines con dynamic state, recarga en caliente en plena ejecución.
- **Hacer las matemáticas 3D que necesito, sin derivarlas**: mis propios `Vec3`/`Mat4` con tests,
  tarjetas encadenadas de derecha a izquierda, perspectiva mediante `w`, look-at, cámara orbital con
  límite de pitch.
- **Leer archivos a mano**: un parser de `.obj` (quads, n-gonos, índices negativos, tokens `1/2/3`,
  ear clipping para caras cóncavas) y un cargador de TGA (24/32 bits, RLE, los dos orígenes) — sin
  librerías.
- **Renderizar más allá de un modelo**: test de profundidad, texturas con sRGB correcto, iluminación
  plana, instancing (512 copias, un draw), un cielo con cube map, cromado, reflejos en vivo
  renderizados a textura, texto en pantalla.
- **Escribir Rust con soltura viniendo de C**: ownership en vez de `free`, `Drop` para el orden de
  destrucción, `Result` + `?`, traits, closures como callbacks, workspaces.
- **Separar engine y app**: un crate `engine` reutilizable que scop usa solo a través de su API
  pública — y las dos preguntas de reutilización para decidir qué va dónde.

## Qué se quedó superficial a propósito

- **Las matemáticas** — limitadas a T1 desde el principio (usarlas, probarlas, no derivarlas);
  §§ 20–25 sin matemáticas en absoluto. Los cuaterniones, la inversa traspuesta para modelos
  estirados y por qué la matriz de perspectiva tiene exactamente esas entradas son nombres
  conocidos, no fórmulas entendidas.
- **Detalle del sistema operativo / driver** — p. ej. Q-3.2 se quedó en una frase (por qué MoltenVK
  necesita la activación de portabilidad), por elección.
- **Rendimiento** — una sonda, buffers de tamaño fijo, `device_wait_idle` allí donde se destruyen
  cosas, vsync siempre activado. Bien para scop; nunca medí el coste real en la GPU.
- **Iluminación** — una luz direccional, sombreado plano, sin sombras, sin especular.
- **Reserva de memoria** — una reserva por buffer; sin allocator.

## Qué fue más difícil de lo que el plan suponía

- **Sincronización (§ 5)** — fotogramas vs slots de fotograma vs imágenes de la swapchain necesitó
  una explicación extra; "quién espera a qué" volvió como fallo en §§ 7, 10 y 16 antes de asentarse
  (limpio en § 19).
- **sRGB (§ 14)** — predije "demasiado oscuro" para una textura `UNORM`; queda demasiado pálida
  (doble codificación). Decodificar al entrar / codificar al salir necesitó variantes nuevas para
  asentarse.
- **Ear clipping (§ 17)** — al principio pensé que la oreja era el trozo que se tira; hizo falta
  trabajar la L a mano. Y el propio archivo de la sección se perdió en un commit y hubo que
  reescribirlo.
- **Tiempo de compilación vs tiempo de ejecución (§ 6)** — los confundí dos veces ("build de
  depuración" ≠ "tiempo de compilación").
- **Disposición de la matriz en memoria (§ 9)** — fila/columna en el papel frente a en memoria, y lo
  que hace de verdad una matriz traspuesta (alimenta `w`, deforma el modelo).
- **Del revés vs boca abajo (§ 19)** y **los cube maps como imágenes, no direcciones (§ 23)** — los
  dos necesitaron otra explicación con un dibujo.
- **El teclado español** — `+`/`−` no se registraban, así que el dolly pasó a W/S.
- **Portabilidad en el campus (§ 16)** — sin `glslc` y sin permisos de administrador: de ahí el
  respaldo `.spv` commiteado y la descarga del SDK de LunarG en `deps.sh`.

## Qué debería retomar el siguiente proyecto

De las líneas "Revisit if" de `DECISIONS.md` y la lista de aplazados de `00-PLAN.md`:

- **ft_vox**: una **cola de borrado** por slot de fotograma (meshes que cambian cada fotograma); un
  **allocator** de memoria de verdad y saltarse el staging con memoria unificada / Resizable BAR;
  **back-face culling** cuando las meshes tengan un winding coherente; **atlas de texturas y
  mipmaps**; buffers de instancias que crecen; redimensionado en vivo más suave; **sombras**.
- **ft_newton**: **cuaterniones** para la rotación libre; meshes cerradas (soldadas) para la física;
  la inversa traspuesta para las normales de modelos estirados.
- **Mejoras de renderizado cuando se quieran**: normales suaves (promediadas) → sombreado suave y
  cromado suave; mapeo triplanar para modelos orgánicos; varias sondas de reflejos; cielos HDR; un
  atlas de fuente más grande para texto con acentos.
- **El propio engine** — cámara, transformaciones, tweens, constructor de meshes, recarga en
  caliente, cielo, shaders por defecto, sonda de reflejos, texto superpuesto — está listo para ser
  la base del engine en Rust.
- **La cola de repaso** aún tiene ~30 elementos con 0 recuerdos limpios: merece una pasada antes de
  la defensa.

## Notas sobre el propio plan

- **Cambiar el método el 2026-11-03 fue la mejor decisión**: responder primero al gate, y luego que
  Claude escribiera el checkpoint y yo lo explicara de vuelta, mantuvo la comprensión sin teclearlo
  todo a mano. Los explain-backs pillaron huecos reales (bind vs write, `+= gap` vs `+= max_step`, el
  orden de las matrices en instancing).
- **Variantes nuevas tras un fallo** funcionaron mejor que releer la respuesta — la mayoría de fallos
  pasaron con la segunda pregunta, redactada de otra forma.
- **Bajar §§ 20 y 22 a T1 / sin matemáticas** encajó con lo que ya sabía de rubik y cubed; las
  preguntas del gate sobre comportamiento y montaje bastaron.
- **Los extras del bonus crecieron a seis secciones (§§ 19–25)** — mucho más de lo que necesita un
  bonus de 10 puntos, a propósito, como base del engine. § 24 (llevarlo todo al engine) fue la forma
  correcta de cerrar.
- **Pausar el espejo en español** ahorró tiempo a mitad de proyecto; ponerlo al día por tandas
  funcionó bien.
- **Lección para la próxima vez**: commitear el archivo de la sección *antes* de marcarla como hecha
  — tres archivos se vaciaron con commits de "completed", y el texto de § 17 hubo que reconstruirlo
  a partir del código.
- **La cola de repaso se llenó pero casi nunca se revisó** — en el próximo plan, programar sesiones
  de repaso en vez de dejarlas para "la próxima sesión".
