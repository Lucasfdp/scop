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
