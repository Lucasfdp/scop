---
section: 21
slug: bonus-shader-hot-reload
tier: T1
status: done
prereqs: [6, 16, 19]
terms: [hot reload, runtime shader compile, modification time, polling, pipeline swap, shader contract]
---

# Sección 21 — Shaders que se recargan en marcha (bonus)

**Profundidad: T1** — casi todo reutilización: § 6 (los shaders se convierten en pipelines),
§ 16 (`glslc`, el respaldo `.spv`), § 19 (lo nuevo antes que lo viejo, esperar antes de
destruir). Extra del bonus (elegido por el propietario, 2026-11-18): **editar un shader y ver
el cambio sin reiniciar**.
**Serás capaz de:** dejar scop abierto, cambiar `mesh.frag` en tu editor, guardar y ver los
colores nuevos un momento después — o, si el shader está roto, leer el error del compilador
mientras la versión vieja sigue funcionando.

---

## La idea

Hasta ahora los shaders se compilan una vez, **antes** de compilar scop (el `build.rs` de
§ 16), y los bytes SPIR-V quedan incrustados en el binario. Cambiar un color significa:
editar, recompilar, reiniciar y volver a encontrar la misma vista. La recarga en caliente
quita los tres últimos pasos.

Son tres trabajos pequeños:

1. **Notar el cambio.** Cada cuarto de segundo, scop le pregunta al sistema de archivos
   cuándo se cambiaron por última vez `mesh.vert` y `mesh.frag` (su **fecha de
   modificación**). Una fecha más nueva que la de la última comprobación = se guardó el
   archivo. Preguntar repetidamente así es **polling** (sondeo).
2. **Compilarlo.** Vulkan solo acepta SPIR-V (§ 6), así que scop ejecuta el mismo `glslc` que
   usa el build — ahora con scop en marcha — y se queda con los bytes que produce.
3. **Cambiar el pipeline.** Un pipeline guarda su propia copia compilada de los shaders (§ 6),
   así que shaders nuevos significan pipelines nuevos. Mismo orden que en § 19: construir
   primero los **nuevos**; solo si ha funcionado, esperar a la GPU y destruir los viejos. El
   pipeline nuevo va al hueco del viejo, así que los `PipelineId` de scop siguen funcionando.

Si algo falla — `glslc` informa de un error, o no se puede construir el pipeline — scop
imprime el mensaje y **sigue dibujando con los shaders viejos**. Una errata nunca cierra el
visor.

**Lo que no puede cambiar: el contrato.** Un shader y el lado de Rust están de acuerdo en qué
entra y qué sale: la disposición del vértice (§ 7), el bloque de la cámara (§ 10), el bloque
de push (§ 15), la textura (§ 14). La recarga en caliente puede cambiar cualquier cosa
*dentro* del shader — colores, fórmulas, los modos de depuración —, pero una entrada nueva que
el lado de Rust no envía sigue necesitando un cambio en Rust y un reinicio (Q-21.4).

**Dónde deja de valer la analogía:** "guardar y verlo" hace que parezca editar una página web.
Pero la recarga solo funciona si `glslc` está instalado: en una máquina sin él (el respaldo
del campus, § 16), scop dice que la recarga en caliente está apagada y sigue con los shaders
incrustados.

---

## Los nombres

**hot reload** — sustituir parte de un programa en marcha (aquí: sus shaders) sin
reiniciarlo.

**runtime shader compile** — ejecutar `glslc` desde dentro de scop
(`std::process::Command`), leyendo su SPIR-V de la salida. La misma herramienta que el build,
en otro momento.

**modification time** — cuándo se escribió por última vez un archivo, según lo registra el
sistema de archivos ("mtime"). Guardar la cambia; leer no.

**polling** — comprobar repetidamente ("¿ha cambiado ya?") en vez de que te avisen. Sencillo,
y barato a cuatro comprobaciones por segundo.

**pipeline swap** — construir el pipeline nuevo, esperar a la GPU, destruir el viejo, poner el
nuevo en su hueco. El id sigue siendo el mismo: "el pipeline sólido de scop", sea cual sea la
versión.

**shader contract** — todo lo que un shader espera que le proporcione el lado de Rust
(entradas de vértice, bloques uniform y de push, texturas). La recarga en caliente funciona
dentro de él, nunca a través de él.

---

## Cómo funciona realmente

### 1. Engine: pipelines a partir de bytes, sustituidos en su sitio

```
Shaders<'a> { vertex: &'a [u8], fragment: &'a [u8] }   // antes &'static: los bytes en ejecución son propios
Gfx::replace_pipeline(id, shaders, style)
    new = Pipeline::new(…)          → ¿error? devolverlo; el pipeline viejo queda intacto
    device_wait_idle                // puede que los fotogramas en vuelo aún usen el viejo (§ 19)
    destruir el viejo, poner el nuevo en el hueco id // mismo id: los resguardos de la app siguen valiendo
```

### 2. scop: vigilar, compilar, cambiar

```
ShaderWatch { folder, last_seen: (mtime vert, mtime frag), since_check }
    folder = scop/shaders, tal como se escribió en el binario al compilar (CARGO_MANIFEST_DIR)
    carpeta ausente o sin glslc → imprimir "hot reload off: …" una vez, no volver a comprobar
update: since_check += dt; ≥ 0.25 s → leer las dos mtimes
        más nuevas que last_seen → recordarlas, y luego:
            glslc mesh.vert -o -  y  glslc mesh.frag -o -    (SPIR-V por stdout)
            error → imprimir el mensaje de glslc (archivo:línea: error …), conservar los pipelines viejos
            ok    → replace_pipeline para cada estilo que tiene scop (relleno, wireframe, puntos)
                    imprimir "shaders reloaded"
```

Los shaders incrustados (`include_bytes!`) siguen siendo con los que arranca scop; el
respaldo `.spv` commiteado no se toca — `make spv` sigue siendo lo que lo actualiza.

### Decisiones

| Elección | Frente a | Por qué |
|---|---|---|
| **Sondear las fechas de modificación cada 0.25 s** | Una librería de vigilancia de archivos (`notify`) | Solo se permiten librerías de ventanas/eventos; `std::fs::metadata` basta |
| **Ejecutar `glslc` en tiempo de ejecución** | Meter un compilador de GLSL dentro de scop | La misma herramienta que el build; nada nuevo que instalar ni escribir |
| **Pipelines nuevos primero, los viejos se conservan si falla** | Destruir y luego construir | La misma regla que § 19: un shader roto nunca deja a scop sin ninguno |
| **Sustituir en el mismo hueco (se conserva el id)** | Id nuevo, la app actualiza sus resguardos | El id significa "este pipeline"; a la app no deberían importarle las versiones (Q-21.5) |
| **Carpeta de fuentes incrustada al compilar** | Un flag de línea de comandos | Funciona desde el repo sin configurar nada; en cualquier otro sitio, la recarga simplemente se apaga |
| **Recarga en caliente en todos los builds** | Solo en builds de depuración | Inofensiva si faltan la carpeta o `glslc`; útil en release para comprobar la tasa de fotogramas |

---

## Trampas conocidas

- **Error de recarga a mitad de guardado** — algunos editores escriben el archivo en dos
  pasos; scop lo pilla a medio escribir y `glslc` se queja. El siguiente guardado (o la
  siguiente comprobación) lo arregla — mientras tanto funcionan los shaders viejos.
- **No pasa nada al guardar** — el editor guardó en otro sitio (una copia, otra carpeta), o la
  recarga en caliente se apagó sola al arrancar: lee las primeras líneas que imprimió scop.
- **Entrada nueva, compila bien, pero la validación se queja** — cambió el contrato del shader
  (Q-21.4). Deshazlo, o haz que el lado de Rust coincida y reinicia.
- **Error de validación al destruir el pipeline viejo** — destruido mientras un fotograma en
  vuelo lo usaba: espera antes (§ 19, Q-19.3).
- **El `.spv` commiteado está desfasado** — la recarga en caliente nunca lo escribe; `make spv`
  antes de commitear (§ 16).
- **`glslc` encontrado al compilar pero no en ejecución** — la ruta de búsqueda del programa es
  otra (p. ej. lanzado desde un lanzador de apps). scop lo dice y sigue.

---

## Checkpoint

**Cumple (subject):** extra del bonus (recarga de shaders en caliente). Método: primero el
gate, luego Claude lo escribe y te lo explica.

1. **Engine:** `Shaders<'a>`; `Gfx::replace_pipeline` (nuevo primero, esperar, destruir el
   viejo, mismo hueco).
2. **scop:** `ShaderWatch` (carpeta, mtimes, sondeo cada 0.25 s, se apaga limpiamente);
   `compile` con `glslc -o -`; recargar el pipeline de cada estilo; mensajes de recargado /
   error / apagado; README.
3. **Tests:** una mtime más nueva se nota exactamente una vez; sin cambios = sin recarga; una
   carpeta ausente apaga la vigilancia; `compile` devuelve el texto de error de `glslc` para un
   shader roto (se salta si `glslc` no está instalado).

**Verificar:** ejecuta scop, cambia `FRONT` en `mesh.frag` a verde, guarda — el modo
orientación se vuelve verde sin reiniciar; añade una errata, guarda — se imprime el error, la
imagen no cambia; arréglalo — recargado; validación callada.

---

## Preguntas

Q-21.1 (recuerdo) ¿Por qué scop tiene que ejecutar `glslc` antes de poder usar el shader
editado — por qué no puede pasarle el archivo `.frag` directamente a Vulkan?

Q-21.2 (aplicación) ¿Por qué construir los pipelines nuevos *antes* de destruir los viejos?

Q-21.3 (predicción) Guardas `mesh.frag` con un punto y coma de menos. ¿Qué ves en pantalla, y
qué en la terminal? ¿Qué pasa cuando lo arreglas y vuelves a guardar?

Q-21.4 (predicción) Añades `layout(location = 4) in vec3 in_tangent;` a `mesh.vert` y guardas.
Compila. ¿Por qué sigue sin funcionar, y qué hace falta para que funcione?

Q-21.5 (aplicación) § 19 nunca reutiliza el hueco de una mesh destruida, pero § 21 pone el
pipeline nuevo en el hueco del viejo, conservando el mismo id. ¿Por qué aquí no pasa nada por
reutilizar el hueco?

Respuestas: `ANSWERS.md` § 21
