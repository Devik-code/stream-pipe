# Evaluación Técnica: Rust Developer - Stream‑Pipe

Instrucciones: Respondo con base en mi experiencia práctica con sistemas de bajo nivel y streaming, usando ejemplos concretos, criterios técnicos y decisiones reales de diseño.

---

## Parte 1

### 1. Errores de segmentación y condiciones de carrera en streaming

**🍎 Analogía**:  
Imagina que dos hilos intentan escribir en el mismo buffer de video al mismo tiempo, como dos personas pintando el mismo lienzo a la vez. En C/C++ podrían pisarse y romper el lienzo (segmentation fault). En Rust, el compilador funciona como un bibliotecario: no te deja modificar el lienzo si alguien más ya lo está usando.

**⚙️ Criterio técnico**:  
Para evitar condiciones de carrera en el streaming, usaría `Arc<Mutex<T>>` sobre el buffer compartido.  
- `Arc` permite que varios hilos compartan el mismo buffer con un contador de referencia atómico.  
- `Mutex` garantiza que solo un hilo pueda obtener una referencia mutable (`&mut T`) a la vez, bloqueando el acceso mientras se escribe.

**💻 Ejemplo en el código**  
```rust
// src/main.rs (Línea 64)

let buffer: SharedBuffer = Arc::new(Mutex::new(VecDeque::new()));
// Arc = múltiples dueños del buffer.
// Mutex = solo un hilo puede escribir a la vez.
```

Con esto, el sistema es seguro ante data races sin necesidad de cuidar manualmente locks o sincronización.

---

### 2. Manejo de errores en paquetes de video vs `unwrap()`

**🍎 Analogía**:  
Usar `unwrap()` es como abandonar un barco ante la primera gotera. En un sistema 24/7, el barco debe seguir flotando aunque haya fallos. Por eso, envuelvo cada operación sensible en un `Result<T, E>`, y el “capitán” decide si reintentar, reconectar o descartar el paquete.

**⚙️ Criterio técnico**:  
Evito `unwrap()` en cualquier operación de red o buffer de video.  
- Uso `Result<T, E>` con propagación de errores mediante `?`.  
- Defino un tipo de error propio con `thiserror` (por ejemplo `VideoError`) para categorizar: errores de red, de buffer, de envenenamiento de candados, etc.  
- En el código de producción, siempre manejo o convierto los errores a un formato que el sistema pueda recuperar sin panic.

**💻 Ejemplo en el código**  
```rust
// src/downloader.rs (Línea 184)

let chunk = chunk?;  // Error controlado, el proceso sigue.
```

Así, un chunk corrupto o tiempo de espera agotado no derrumba el sistema completo, sino que se gestiona localmente.

---

### 3. Vicios de gestión de memoria en C/C++ y cómo Rust los corrige

**🍎 Analogía**:  
En C/C++ es fácil crear un “puntero fantasma”: apuntas a un cuadro de video que ya fue liberado. En streaming, esto se traduce en leer bytes de memoria ya borrada, con resultados impredecibles. Rust funciona como un inspector: cuando el dueño de la memoria se va, quita la llave, imposibilitando el acceso.

**⚙️ Criterio técnico**:  
Los vicios más peligrosos son:
- `use‑after‑free` y `double‑free`,  
- puntos de acceso inconsistentes a colecciones compartidas.  

Rust los resuelve con:
- **Ownership + RAII**: el dueño de la memoria es único, y el `Drop` libera automáticamente sin que el sistema cierre el “lienzo” mientras alguien lo está usando.  
- **Borrow checker**: durante el compile time, el sistema impide que existan referencias vivas a memoria que ya fue liberada.

**💻 Ejemplo en el código**  
```rust
// src/downloader.rs (Línea 204)

let frame_bytes = accumulator.split_to(frame_size);
```
Aquí `frame_bytes` es el nuevo dueño de esos datos. No hay riesgo de liberar la memoria mientras otra parte del sistema la está procesando.

---

## Parte 2

### 1. Sistema de ownership, borrowing y mutabilidad

**🍎 Analogía**:  
Piensa en el buffer de video como un documento original:
- **Ownership**: solo un dueño real, cuando se va, el documento se destruye.  
- **& (inmutable)**: puedes sacar fotocopias que todos pueden leer.  
- **&mut (mutable)**: solo una persona puede editar el documento, y durante eso, las copias de lectura se bloquean.  

Si nadie puede leer mientras alguien está escribiendo, las condiciones de carrera se vuelven imposibles de compilar.

**⚙️ Criterio técnico**:  
Rust impone:
- Infinitas referencias inmutables (`&T`).  
- Máximo una referencia mutable (`&mut T`).  
- Nunca ambas sobre el mismo dato al mismo tiempo.  

Esto elimina `data races` en tiempo de compilación, incluso en entornos de alto paralelismo como streaming de video.

**💻 Ejemplo en el código**  
```rust
// src/main.rs (Línea 70)

let buffer_prod = Arc::clone(&buffer);
// El buffer original queda protegido; la modificación solo pasa
// cuando se adquiere el Mutex<T> en el hilo correspondiente.
```

---

### 2. Uso de `Result` y `Option` y cuándo usar `unwrap()`

**🍎 Analogía**:  
- `Result`: una caja que puede contener un premio (`Ok`) o una factura (`Err`).  
- `Option`: una caja que puede estar llena (`Some`) o vacía (`None`).  
- `unwrap()` es como abrir la caja con un martillo: si estaba vacía o era una factura, el programa se rompe. Solo lo uso cuando el sistema ha verificado 100 % que el valor está presente (por ejemplo, inmediatamente después de haberlo insertado).

**⚙️ Criterio técnico**:  
- Uso `match` o `if let` para manejar exhaustivamente ambos casos.  
- Evito `unwrap()` en:
  - entrada de red,  
  - configuración,  
  - dependencias externas.  
- Solo lo permito en pruebas o en invariantes donde una condición de error sería lógicamente imposible (por ejemplo, valores internos que acabo de construir).

**💻 Ejemplo en el código**  
```rust
// src/pipeline.rs (Línea 89)

match frame {
    Some(f) => { /* Procesa el frame */ },
    None => { /* El canal se cerró; limpia recursos */ }
}
```

---

### 3. Concurrencia con `Arc`, `Mutex` y `Send`/`Sync`

**🏦 Analogía: Club y caja fuerte**
- `Arc`: el carnet del club, permite que muchos hilos entren.  
- `Mutex`: la caja fuerte, solo uno puede abrirla a la vez para escribir.  
- `Send`: capacidad de llevar el carnet a otro club (transferir ownership).  
- `Sync`: capacidad de que muchos vean el mismo objeto a través de referencias inmutables.

**⚙️ Criterio técnico**:  
- `Arc<T>` ofrece conteo de referencias atómico entre hilos.  
- `Mutex<T>` permite “interior mutability” con exclusión mutua.  
- `Send` y `Sync` son traits de marca que Rust usa para garantizar que un tipo:
  - puede ser movido entre hilos (`Send`),  
  - puede ser compartido inmutablemente entre hilos (`Sync`).  

**💻 Ejemplo en el código**  
```rust
// src/downloader.rs (Línea 69)

pub type SharedBuffer = Arc<Mutex<VecDeque<Frame>>>;
```

---

### 4. Errores de memoria en C/C++ y cómo Rust los evita

**🍎 Analogía**:  
En C++ puedes crear una lista de frames y luego liberarla mientras un hilo todavía está leyendo de ella. En Rust, si intento hacer eso, el compilador se niega y exige que el sistema mantenga la estructura viva mientras haya referencias activas sobre ella.

**⚙️ Criterio técnico**:  
En C++ una fuente común de bugs es la **invalidación de iteradores**: modificar una colección mientras se está iterando. Rust detecta esto en tiempo de compilación gracias al borrow checker, que prohíbe tener un préstamo mutable (`&mut T`) mientras existen préstamos inmutables activos (`&T`).

**💻 Ejemplo en el código**  
```rust
// src/pipeline.rs (Líneas 116–121)

let header: Vec<String> = f
    .data
    .iter()
    .take(4)
    .map(|b| format!("{:02x}", b))
    .collect();
```
Aquí Rust garantiza que `f.data` no será modificado ni liberado mientras el iterador está recorriendo los primeros 4 bytes, evitando un acceso a memoria incoherente.

---

### 5. Consideraciones de portabilidad entre Linux y Windows

**🍎 Analogía**:  
Escribir un programa para Linux y Windows es como redactar un libro pensado para España y México, cuidando diferencias de vocabulario. En Rust, uso construcciones estándar que se adaptan automáticamente a cada plataforma.

**⚙️ Criterio técnico**:  
- Uso `std::path::PathBuf` y `std::path::Path` para manejar rutas sin preocuparme por `/` vs `\`.  
- Para configuraciones y logs, uso rutas estándar, como `/var/lib/stream-pipe` en Linux y directorios de aplicación en Windows.  
- En proyectos que requieren compilación cruzada, empleo `cross` (basado en Docker) para generar binarios Windows y Linux desde una sola máquina, incluyendo soportes ARM si es necesario para embebidos.

**💻 Ejemplo en el código**  
```rust
// src/configuration.rs (Líneas 100–105)

.add_source(File::with_name("/var/lib/stream-pipe/config.toml").required(false))
.add_source(File::with_name("config.toml").required(false))
```
Así el sistema busca primero una ruta típica de Linux, luego una ruta local útil para desarrollo en Windows.

---

### 6. Pruebas de estrés para streaming de video

**🍎 Analogía**:  
Primero probás que cada plato del restaurante sale bien por separado (prueba unitaria), luego que cocina y salón funcionan juntos (integración), y finalmente simulás 300 comensales a la vez para ver dónde colapsa (estrés). En Rust cada nivel tiene su herramienta.

**⚙️ Criterio técnico**:  
- `cargo test` + `#[test]` es la base: Rust incluye el framework de pruebas integrado, sin dependencias externas. Ya existe en el proyecto en `src/configuration.rs`.  
- Para código async (asíncrono — no bloquea el hilo mientras espera), uso `#[tokio::test]`: levanta un runtime (entorno de ejecución) de Tokio solo para esa prueba.  
- Para estrés, lanzo múltiples workers con `JoinSet` (conjunto de tareas — agrupa tasks async) contra el mismo buffer y verifico que el total de bytes procesados sea exacto sin importar cómo se distribuya la carga.

**💻 Ejemplo en el código**  
```rust
// src/pipeline.rs — tests reales del proyecto (corren con `cargo test`)

// Test 1 — un frame: verifica que el consumidor procesa y devuelve bytes exactos
#[tokio::test]
async fn test_consumer_frame_unico() { ... }  // assert_eq!(total, 64)

// Test 2 — buffer vacío: verifica que el consumidor sale limpiamente sin loop infinito
#[tokio::test]
async fn test_consumer_buffer_vacio_done() { ... }  // assert_eq!(total, 0)

// Test 3 — varios frames: verifica el total con frames de distintos tamaños
#[tokio::test]
async fn test_consumer_multiples_frames() { ... }  // assert_eq!(total, 600)

// Test 4 — estrés: 8 workers en paralelo sobre 500 frames, ningún frame se pierde
#[tokio::test]
async fn test_stress_multiples_workers() { ... }  // assert_eq!(total, 32_000)
```

```bash
cargo test                               # corre los 6 tests del proyecto
cargo test test_stress                   # corre solo el test de estrés
cargo test -- --nocapture                # muestra logs dentro de los tests
```

Los tres niveles avanzados también están implementados en el proyecto:

- **`wiremock`** → `src/downloader.rs`: levanta un servidor HTTP local falso y verifica que el downloader maneja correctamente una descarga exitosa y un error 404.
- **`proptest`** → `src/pipeline.rs`: genera listas aleatorias de entre 1 y 50 frames con tamaños de 1 a 1024 bytes y verifica que el total siempre sea exacto.
- **`criterion`** → `benches/pipeline_bench.rs`: mide el rendimiento de `consume()` con 10, 100 y 1000 frames, y con tamaños de frame de 4 KB, 32 KB y 64 KB.

```bash
cargo test          # corre los 9 tests (unitarios + wiremock + proptest)
cargo bench         # corre los benchmarks y genera reporte en target/criterion/
```