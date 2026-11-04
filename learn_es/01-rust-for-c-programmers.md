---
section: 1
slug: rust-for-c-programmers
tier: T2
status: not-started
prereqs: []
terms: [ownership, move, borrow, lifetime, Option, Result, ? operator, trait, Drop, RAII, crate, Cargo workspace, unsafe, raw pointer, CString, repr(C)]
---

# Sección 1 — Rust para quien viene de C

**Profundidad: T2** — todas las secciones siguientes están escritas en Rust, y los
tiempos de vida de los objetos de Vulkan encajan directamente con las reglas de
ownership de Rust. Un modelo flojo aquí convierte cada bug gráfico en una pelea con
el compilador.
**Serás capaz de:** leer y escribir el Rust que necesitas para scop sin que el borrow
checker te tape el problema gráfico real, y montar el workspace donde vivirá todo el
proyecto.

**Lee en paralelo:** *The Rust Programming Language* ("el Libro"), capítulos 1–10
(gratis en doc.rust-lang.org/book). Esta sección es el mapa; el Libro es el territorio.
Aquí no se enseña la *sintaxis* de Rust — el Libro lo hace bien. Esta sección cubre las
ideas que son distintas de C y las que Vulkan va a usar.

---

## La idea

En C, cada puntero es una promesa que te haces a ti mismo: "esta memoria sigue viva y
nadie la está cambiando ahora". El compilador te cree. Cuando te equivocas, te enteras
en ejecución — un segfault, un double free o, peor, un valor corrompido en silencio.

Rust mete esas promesas en el sistema de tipos. Cada valor tiene exactamente **un
dueño** — una variable responsable de él. Cuando el dueño sale de su ámbito, el valor se
limpia solo, como un `free()` que el compilador escribe por ti en la llave de cierre.
Si le das el valor a otro, la propiedad se *mueve*: la variable antigua ya no se puede
usar, así que no hay forma de liberarlo dos veces.

También puedes *prestar* un valor sin darlo. Un préstamo es o bien:
- **muchos lectores a la vez**, o
- **exactamente un escritor, y ningún lector.**

Nunca las dos cosas. Esa única regla hace imposibles en Rust normal las carreras de
datos y los bugs de "alguien me lo cambió por debajo" — y se comprueba al compilar, no
al ejecutar.

Los errores también funcionan distinto. En lugar de devolver `-1` y poner `errno`, una
función de Rust que puede fallar devuelve un valor que es *o bien* el resultado *o
bien* el error, y el compilador no te deja usar el resultado sin decidir qué haces con
el error.

Cuando de verdad necesitas la libertad de C — y hablando con Vulkan, una API de C, la
necesitarás — escribes `unsafe { ... }`. Eso no apaga las reglas; marca un bloque
pequeño donde *tú* recuperas la responsabilidad de las promesas, igual que en C.

---

## Los nombres

**ownership** (propiedad) — la regla de que cada valor tiene exactamente una variable
responsable de limpiarlo.

**move** (mover) — pasar la propiedad a otra variable o función. Tras un move, la
variable original no se puede usar. Para tipos simples copiables (`i32`, `f32`, `bool`)
se hace una *copia* y ambas siguen siendo válidas.

**borrow** (préstamo) — prestar un valor sin transferir la propiedad. `&T` es un
*préstamo compartido* (solo lectura, se permiten muchos); `&mut T` es un *préstamo
mutable* (lectura-escritura, solo uno y sin préstamos compartidos a la vez). La parte del
compilador que lo comprueba es el **borrow checker**.

**lifetime** (tiempo de vida) — cuánto dura un préstamo. Normalmente el compilador lo
deduce; si no puede, lo escribes como `'a`. La regla: un préstamo nunca puede vivir más
que el valor al que apunta — así que los punteros colgantes no compilan.

**`Option<T>`** — "quizá un valor": o `Some(valor)` o `None`. Sustituye a NULL.

**`Result<T, E>`** — "un valor o un error": o `Ok(valor)` o `Err(error)`. Sustituye a
los códigos de retorno y a `errno`.

**`?` operator** (operador `?`) — se escribe tras una expresión que devuelve un `Result`:
"si es un error, devuélvelo desde mi función ya; si no, dame el valor". El equivalente en
Rust de `if (ret < 0) return ret;`, en un carácter.

**trait** — un conjunto de funciones que un tipo promete ofrecer (como una interfaz).
`Debug` (imprimible con `{:?}`), `Clone`, `Drop` son traits.

**`Drop`** — el trait cuya única función `drop()` se ejecuta automáticamente cuando el
dueño de un valor sale de su ámbito. Tu destructor.

**RAII** — **R**esource **A**cquisition **I**s **I**nitialisation ("adquirir el recurso es
inicializar"). Un nombre torpe de C++ para una idea simple: atar un recurso (memoria, un
fichero, un objeto de Vulkan) a la vida de un valor, de modo que crear el valor adquiere
el recurso y destruir el valor lo libera. El nombre describe la primera mitad y olvida la
importante, la segunda — no intentes sacarle lógica.

**crate** — la unidad de compilación de Rust: una librería o un binario. Más o menos "un
`.a` o un ejecutable".

**Cargo workspace** — varios crates en un repositorio que comparten directorio de build y
lock file. **Cargo** es la herramienta de build y gestor de paquetes de Rust (piensa en
`make` + un gestor de paquetes en uno).

**`unsafe`** — un bloque o función donde puedes hacer cinco cosas que el compilador no
puede comprobar: desreferenciar punteros crudos, llamar funciones `unsafe` (todas las de
Vulkan), acceder a statics mutables, implementar traits `unsafe`, acceder a campos de
`union`.

**raw pointer** (puntero crudo) — `*const T` / `*mut T`. Exactamente un puntero de C: sin
propiedad, sin lifetime, puede ser nulo o colgante. Crearlo es seguro; *usarlo* es `unsafe`.

**`CString`** — un string con propiedad y `\0` final, con la forma de un `char *` de C. El
`String` normal de Rust *no* termina en nulo, así que lo necesitas cada vez que una API de
C (Vulkan) pide un nombre.

**`repr(C)`** — **repr**esentation C (representación C). Un atributo (`#[repr(C)]`) que
obliga a que los campos de un struct sigan el orden y la disposición de C. Sin él, Rust
puede reordenar campos. Lo necesitarás para todo lo que la GPU lee byte a byte (vértices,
uniforms).

---

## Cómo funciona realmente

### 1. Moves frente a copias

```rust
// THROWAWAY — not part of scop
let a = vec![1, 2, 3];   // a es dueña de un buffer en el heap
let b = a;               // la propiedad pasa a b; el buffer no se copia
// println!("{:?}", a);  // error de compilación: borrow of moved value: `a`
println!("{:?}", b);     // bien
```

En C, `b = a` con un puntero daría dos punteros a un mismo buffer y, dos `free()`
después, un double free. Rust, en cambio, inutiliza `a`.

Los tipos baratos que no poseen nada (`i32`, `f32`, `[f32; 3]`, tus structs marcados
`#[derive(Clone, Copy)]`) se copian en vez de moverse. Tu futuro `Vec3` será `Copy`. Un
struct que envuelve un objeto de Vulkan **no** debe serlo — copiarlo significaría dos
dueños destruyendo el mismo objeto de la GPU.

### 2. Préstamos: la regla "un escritor XOR muchos lectores"

```rust
// THROWAWAY — not part of scop
let mut v = vec![1, 2, 3];
let first = &v[0];       // préstamo compartido
v.push(4);               // error: cannot borrow `v` as mutable because it is also borrowed as immutable
println!("{first}");
```

¿Por qué es un error? `push` puede realocar el buffer, dejando `first` apuntando a memoria
liberada. En C esto compila y se convierte en un use-after-free. Rust se niega porque
`first` sigue en uso en la última línea.

Las firmas de las funciones te dicen qué tipo de préstamo necesitan:
- `fn len(&self)` — lee
- `fn push(&mut self, x)` — escribe
- `fn into_bytes(self)` — toma la propiedad (consume el valor)

### 3. Errores: `Result` y `?`

```rust
// THROWAWAY — not part of scop
use std::fs;

fn read_config(path: &str) -> Result<String, std::io::Error> {
    let text = fs::read_to_string(path)?;   // si falla: devuelve Err(...) desde read_config
    Ok(text.trim().to_string())
}
```

`?` solo funciona dentro de una función que a su vez devuelve un `Result` (u `Option`). En
`main`, puedes devolver `Result<(), Box<dyn std::error::Error>>` para que `?` funcione ahí
también — `Box<dyn Error>` significa "cualquier tipo de error, en el heap".

`.unwrap()` significa "revienta si esto es un error". Vale para experimentos rápidos; en
scop, un `.obj` que no existe debe dar un mensaje limpio, no un panic.

### 4. `Drop` y el orden de destrucción — aquí es donde Vulkan muerde

Cuando se destruye un struct, Rust destruye sus **campos en el orden en que están
declarados**, de arriba abajo. Las variables locales se destruyen en orden **inverso** al
de declaración.

Los objetos de Vulkan tienen un orden de destrucción estricto: hay que destruir la
swapchain antes que el dispositivo que la creó, y el dispositivo antes que la instancia. Si
un struct de Rust tiene `instance` declarado encima de `device` y confías en los drops
automáticos, la instancia se va primero — y la validation layer (§ 3) te gritará.

Lo resolverás en § 3 con una implementación explícita de `Drop` que destruye las cosas en
el orden correcto. De momento, recuerda: **los campos se destruyen en orden de
declaración, las variables locales en orden inverso.**

### 5. `unsafe` y punteros crudos (T1 — ya conoces los punteros)

Toda función de `ash` que habla con Vulkan es `unsafe`, porque Vulkan recibe punteros
crudos y el compilador no puede comprobarlos. El patrón que usarás constantemente:

```rust
// THROWAWAY — not part of scop
use std::ffi::CString;

let name = CString::new("scop").unwrap();   // con propiedad, termina en nulo
let ptr: *const std::os::raw::c_char = name.as_ptr(); // puntero crudo prestado
// ... pasa ptr a una API de C mientras `name` sigue vivo ...
```

La regla que debes hacer cumplir tú: **el dueño debe vivir más que cualquier puntero crudo
a él.** El borrow checker no rastrea punteros crudos.

Mantén los bloques `unsafe` pequeños y envuélvelos: una función segura que hace bien la
llamada unsafe, para que el resto del código nunca vea un puntero crudo. Esa es la
columna vertebral del crate del motor.

### 6. Crates, módulos y el workspace

```
scop/
├── Cargo.toml            # [workspace] members = ["engine", "scop"]
├── engine/               # crate librería — reutilizable en ft_newton, ft_vox...
│   ├── Cargo.toml
│   └── src/lib.rs
└── scop/                 # crate binario — el proyecto de 42 en sí
    ├── Cargo.toml        # depende de engine = { path = "../engine" }
    └── src/main.rs
```

- `mod foo;` en `lib.rs` incluye `src/foo.rs` (o `src/foo/mod.rs`).
- Todo es **privado por defecto**; `pub` lo hace visible fuera del módulo. Es tu
  herramienta para mantener pequeña la API pública del motor.
- `cargo build` en la raíz compila todo; `cargo run -p scop -- args` ejecuta el binario
  con argumentos.

---

## Trampas conocidas

- **`error[E0382]: borrow of moved value`** — usaste una variable después de moverla.
  Presta (`&x`) en lugar de mover, o haz `.clone()` si de verdad necesitas dos copias.
- **`error[E0502]: cannot borrow ... as mutable because it is also borrowed as immutable`**
  — un préstamo compartido sigue vivo donde intentas mutar. Normalmente se arregla
  terminando la lectura antes de escribir, o copiando el valor antes.
- **Puntero colgante de `CString`** — `CString::new("x").unwrap().as_ptr()` en una sola
  expresión crea un temporal que se destruye al final de la sentencia. El puntero cuelga
  desde ya. Compila bien; revienta o da basura en ejecución. Guarda siempre el `CString`
  en una variable primero.
- **Structs autorreferenciales** — un struct que guarda un valor y a la vez una referencia
  dentro de ese valor no compila. Si te pasa, el problema es el diseño: guarda valores con
  propiedad o índices, no referencias a campos hermanos.
- **Pelearte con el borrow checker a base de `.clone()`** — señal de que no has decidido
  quién es el dueño. Pregúntate "¿quién es dueño de esto?" antes de añadir un clone.
- **`unwrap()` sobre la entrada del usuario** — un fichero que no existe da un panic con
  backtrace. El evaluador probará una ruta equivocada.

---

## Checkpoint

Monta el esqueleto donde vivirá todo el proyecto. **Cumple:** la estructura del proyecto
y el inicio de "parsear un fichero `.obj`" (leerlo).

1. Crea el workspace tal cual en "Cómo funciona realmente" § 6: un `Cargo.toml` raíz con
   `[workspace]`, un crate librería `engine` y un crate binario `scop` que depende de
   `engine`.
2. En `engine`, escribe una función pública que recibe una ruta y devuelve
   `Result<String, SomeErrorType>` con el contenido del fichero. Elige tu tipo de error
   (`std::io::Error` vale por ahora).
3. En `scop`, lee la ruta de los argumentos de línea de comandos (`std::env::args`), llama
   a la función del motor y:
   - imprime el número de líneas si va bien,
   - imprime un error limpio de una línea y sal con código distinto de cero si falla —
     **sin panic, sin `unwrap()`** sobre la entrada del usuario,
   - imprime un mensaje de uso si no hay argumento.
4. Ejecuta `cargo clippy` y arregla todos los warnings.

**Verifica:** `cargo run -p scop -- resources/42.obj` imprime un número de líneas;
`cargo run -p scop -- nope.obj` imprime un error y `echo $status` (fish) muestra un valor
distinto de cero; `cargo run -p scop` imprime el uso.

---

## Preguntas

Q-1.1 (recall) ¿Qué diferencia hay entre `&T` y `&mut T`, y cuántos de cada uno pueden
existir a la vez para un mismo valor?

Q-1.2 (prediction) Esto compila en C y revienta en ejecución. ¿Compila la versión en Rust?
Si no, ¿qué regla lo impide y qué bug de C previene esa regla?
```rust
let mut names = vec![String::from("cube")];
let first = &names[0];
names.push(String::from("teapot"));
println!("{first}");
```

Q-1.3 (prediction) Un struct está declarado como
`struct Gpu { instance: Instance, device: Device }`, ambos con implementaciones de `Drop`
que destruyen el objeto de Vulkan. Cuando se destruye un `Gpu`, ¿qué se destruye primero?
¿Por qué es un problema para Vulkan?

Q-1.4 (application) Tu función devuelve `Result<Mesh, String>`. Dentro llamas a
`fs::read_to_string(path)`, que devuelve `Result<String, std::io::Error>`. ¿Por qué
`fs::read_to_string(path)?` no compila tal cual, y cuál es una forma de arreglarlo?

Q-1.5 (prediction) ¿Qué falla en esta línea, aunque compile?
```rust
let p = CString::new("scop").unwrap().as_ptr();
```

Respuestas: `ANSWERS.md` § 1
