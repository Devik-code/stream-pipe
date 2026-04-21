// ═══════════════════════════════════════════════════════════════
// ARCHIVO: pipeline.rs — CONSUMIDOR (el que procesa la mercadería)
//
// Peras y manzanas:
//   Este archivo es el INSPECTOR DE CALIDAD de la fábrica.
//   Su trabajo es agarrar cajas (frames — cuadros) de la cinta
//   transportadora (buffer — memoria intermedia), abrirlas,
//   ver qué tienen adentro e imprimir un reporte.
//
//   Trabaja AL MISMO TIEMPO que el camionero (downloader.rs):
//   mientras el camionero trae más cajas, el inspector procesa las que ya llegaron.
//
// FLUJO (orden de ejecución):
//  [VIENE DE] main.rs línea 93 → tokio::spawn lanza esta función
//      │        (corre en paralelo — al mismo tiempo — con downloader.rs)
//      │
//      ├─► [PASO 1] intenta sacar un Frame (cuadro) del buffer (cinta)
//      ├─► [PASO 2a] si hay Frame → procesa e imprime datos
//      ├─► [PASO 2b] si la cinta está vacía y el camionero sigue trabajando → esperar
//      └─► [PASO 2c] si la cinta está vacía y el camionero terminó → salir
//
//  [VIENE DE] buffer compartido ← lo llena downloader.rs
//  [VA HACIA]  main.rs → devuelve el total de bytes procesados
// ═══════════════════════════════════════════════════════════════

use std::sync::atomic::Ordering; // Ordering (orden de memoria): controla cómo se ven
// las escrituras entre hilos (threads — unidades de ejecución paralela)
use tracing::{debug, info};

use crate::downloader::{SharedBuffer, SharedDone}; // los tipos que definimos en downloader.rs
use crate::error::AppError; // el tipo de error centralizado del programa

// ── [ENTRADA DESDE main.rs] ───────────────────────────────────────────────
// consume() es llamada por tokio::spawn (lanzador de tareas concurrentes)
// desde main.rs línea 93.
//
// Por qué es `async` (asíncrona — no bloquea el hilo mientras espera)?
//   Cuando la cinta (buffer — memoria intermedia) está vacía pero el camionero
//   (downloader.rs) aún está trabajando, el inspector tiene que ESPERAR.
//   Si esperara de forma bloqueante (blocking — sin ceder el hilo),
//   el hilo (thread — unidad de ejecución) quedaría congelado y el camionero
//   nunca podría avanzar — ambos se esperarían mutuamente para siempre.
//   Eso se llama deadlock (bloqueo mortal — situación donde dos o más partes
//   se bloquean mutuamente y ninguna puede continuar).
//
//   Con `async` (asíncrona) + yield_now().await (ceder el turno ahora):
//   el inspector dice "no hay cajas todavía, te devuelvo el hilo, volvé cuando tengas".
//   Tokio (el runtime — entorno de ejecución) le da el turno al camionero,
//   que avanza y pone más cajas. Luego vuelve al inspector. Nadie se bloquea.
//
// Retorna (devuelve): Result<usize, AppError>
//   usize  = número entero positivo — total de bytes procesados
//   AppError = error si algo falla (candado envenenado, etc.)
pub async fn consume(buffer: SharedBuffer, done: SharedDone) -> Result<usize, AppError> {
    let mut total_bytes = 0usize; // acumulador del total de bytes procesados
    let mut frames_consumed = 0usize; // contador de frames (cuadros) procesados

    // loop = bucle infinito — repetir para siempre hasta que hagamos `break` (salir del bucle)
    info!("Consumer: Entering the main loop (waiting for frames)...");
    loop {
        // ── [PASO 1] Intentar sacar un Frame de la cinta ──────────────
        //
        // Usamos un bloque { } explícito para liberar el candado (Mutex —
        // candado de exclusión mutua) ANTES de procesar el frame (cuadro).
        //
        // Peras y manzanas:
        //   Agarrás la caja de la cinta (lock — tomar el candado),
        //   la apartás hacia vos (pop_front — sacar de la cola),
        //   y SOLTÁS la cinta (el MutexGuard — guardia del candado sale de scope).
        //   Recién AHÍ abrís la caja para inspeccionarla.
        //   Si la inspeccionaras con la mano en la cinta, bloquearías al camionero
        //   que quiere poner más cajas.
        //
        // El MutexGuard (guardia del candado) se destruye automáticamente
        // al llegar a la llave } que cierra el bloque — sin unlock() explícito.
        let frame = {
            buffer
                .lock() // tomar el candado — espera si otro hilo lo tiene
                .map_err(|_| AppError::LockPoisoned)? // convertir PoisonError (error de envenenamiento) → AppError
                .pop_front() // sacar el primer Frame del frente de la cola (FIFO — primero en entrar, primero en salir)
            // ◄── acá termina el bloque { }: MutexGuard (guardia) destruido, candado liberado
            //     ──► downloader.rs puede volver a usar el buffer (memoria intermedia)
        };

        // match = comparar el valor contra varios casos posibles.
        // Peras y manzanas: es como abrir la caja y ver qué hay adentro:
        //   Some(f) → hay un frame adentro → procesarlo
        //   None    → la caja estaba vacía (buffer — cinta vacía)
        match frame {
            // ── [PASO 2a] Llegó un Frame: procesarlo ──────────────────
            Some(f) => {
                // Acumulamos el tamaño (size — bytes) de este frame al total.
                // += es "sumar y guardar": total_bytes = total_bytes + f.size
                total_bytes += f.size;
                frames_consumed += 1;

                // Leemos los primeros 4 bytes del frame como haría un decoder
                // (decodificador — programa que convierte bytes en imágenes) real.
                //
                // Peras y manzanas:
                //   Un decoder H.264 (formato de compresión de video más común) mira
                //   los primeros bytes de cada frame para saber qué tipo es:
                //   - Keyframe (cuadro clave — imagen completa, independiente)
                //   - P-frame  (cuadro predictivo — solo guarda los cambios respecto al anterior)
                //   - B-frame  (cuadro bidireccional — cambios respecto al anterior Y al siguiente)
                //   Nosotros solo los imprimimos en hex (hexadecimal — sistema numérico en base 16,
                //   donde los dígitos van del 0 al 9 y luego A B C D E F,
                //   usado porque cada byte — 8 bits — se representa con exactamente 2 dígitos hex).
                //
                // .iter()     = crear un iterador (recorredor) sobre los bytes del frame
                // .take(4)    = tomar solo los primeros 4 elementos
                // .map(...)   = transformar cada byte en su representación hex como texto
                // {:02x}      = formato: 2 dígitos hex (hexadecimal) con cero a la izquierda si hace falta
                //               ej: el número 5 se muestra como "05", el número 255 como "ff"
                // .collect()  = juntar todos los textos en un Vec<String> (lista de textos)
                let header: Vec<String> = f
                    .data
                    .iter()
                    .take(4)
                    .map(|b| format!("{:02x}", b))
                    .collect();

                debug!(
                    "  Consumed frame #{:>4} | {:>8} bytes | header: [{}] | total: {} bytes",
                    // {:>4} = alinear a la derecha en 4 caracteres (para que quede prolijo en pantalla)
                    f.index + 1, // mostramos desde 1 (no desde 0) porque es más natural para leer
                    f.size,      // cantidad de bytes en este frame
                    header.join(" "), // .join(" ") une los textos con espacio: "00 00 00 20"
                    total_bytes  // total acumulado desde el inicio
                );
                // ──► vuelve al inicio del loop (bucle) a buscar el siguiente Frame
            }

            // ── [PASO 2b / 2c] La cinta estaba vacía ─────────────────
            None => {
                // Revisamos si el camionero (downloader.rs) ya terminó.
                //
                // done.load(Ordering::Acquire):
                //   .load(...) = leer el valor actual del AtomicBool (bandera atómica — true o false)
                //
                //   Ordering::Acquire (orden de adquisición):
                //   Es el par de Ordering::Release (orden de liberación) que usa downloader.rs.
                //   Peras y manzanas:
                //     Release es como firmar el remito al terminar de descargar.
                //     Acquire es como leer ese remito firmado.
                //     Si ves el remito firmado (done=true), entonces TODOS los frames
                //     que el camionero cargó antes de firmar ya están visibles para vos.
                //     Sin este par Release/Acquire, podrías ver done=true pero no ver
                //     los últimos frames — el procesador reordenaría las instrucciones.
                if done.load(Ordering::Acquire) {
                    // ── [PASO 2c] Camionero terminó + cinta vacía → salir ──
                    // No van a llegar más frames — terminamos.
                    // break = romper el loop (bucle), salir de él.
                    // ──► el control va a la línea después del loop
                    break;
                }

                // ── [PASO 2b] Camionero aún trabaja → ceder el turno ──
                //
                // yield_now().await:
                //   yield_now() = "cedé el turno ahora" — devolvé el hilo (thread) al runtime
                //   (entorno de ejecución) de tokio para que otras tareas puedan correr.
                //   .await = esperar hasta que tokio nos dé el turno de nuevo.
                //
                // Sin esto haríamos busy-wait (espera activa — girar en el loop vacío
                // consumiendo el 100% del CPU sin hacer trabajo útil).
                // Peras y manzanas:
                //   En vez de quedarte parado mirando la cinta vacía sin parar,
                //   le decís a tu jefe (tokio) "avisame cuando llegue algo"
                //   y vas a tomar un café. El jefe te llama cuando el camionero
                //   pone algo en la cinta.
                tokio::task::yield_now().await;
                // ──► tokio devuelve el turno acá cuando haya trabajo disponible
            }
        }
    }

    // ── [SALIDA → main.rs] ────────────────────────────────────────────
    info!(
        "Consumer finished. Frames: {}, Total bytes: {}",
        frames_consumed, total_bytes
    );

    // Ok(total_bytes) = éxito, devolvemos el total de bytes procesados.
    // main.rs lo recibe en: let total = consumer_result...?;
    Ok(total_bytes)
    // [FIN DE pipeline.rs] ──► main.rs recibe Ok(total_bytes) en tokio::join!
}

// ── Tests (pruebas automatizadas) ─────────────────────────────────────────
//
// #[cfg(test)] = este bloque solo se compila cuando corrés `cargo test`.
// No afecta el binario (ejecutable) de producción.
//
// Correr todos los tests:       cargo test
// Correr un test específico:    cargo test test_consumer_frame_unico
// Ver output de los tests:      cargo test -- --nocapture
#[cfg(test)]
mod tests {
    use super::*; // importar todo lo del módulo padre (consume, SharedBuffer, etc.)
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use bytes::Bytes;
    use proptest::prelude::*;

    use crate::frame::Frame;

    // helper (función auxiliar — ayudante): crea un Frame con datos de ceros.
    // Peras y manzanas: es como una caja de relleno para los tests —
    // no nos importa el contenido, solo el tamaño.
    fn frame_vacio(index: usize, size: usize) -> Frame {
        Frame {
            index,
            size,
            data: Bytes::from(vec![0u8; size]),
        }
    }

    // ── Test 1: un solo frame ─────────────────────────────────────────────
    //
    // Verifica que el consumidor procesa correctamente un único frame
    // y devuelve el total de bytes exacto.
    #[tokio::test]
    async fn test_consumer_frame_unico() {
        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let done = Arc::new(AtomicBool::new(false));

        // Insertamos un frame de 64 bytes directo en el buffer — sin red real
        buffer.lock().unwrap().push_back(frame_vacio(0, 64));

        // done=true: le decimos al consumidor que no van a llegar más frames
        done.store(true, Ordering::Release);

        let total = consume(buffer, done).await.unwrap();

        // assert_eq! = "afirmar que son iguales" — si no, el test falla con mensaje claro
        assert_eq!(total, 64);
    }

    // ── Test 2: buffer vacío con done=true ────────────────────────────────
    //
    // Verifica que el consumidor sale limpiamente cuando el buffer está vacío
    // y el productor ya terminó — sin quedarse en un loop infinito.
    #[tokio::test]
    async fn test_consumer_buffer_vacio_done() {
        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let done = Arc::new(AtomicBool::new(true)); // done=true desde el inicio

        let total = consume(buffer, done).await.unwrap();

        assert_eq!(total, 0); // nada que procesar → 0 bytes
    }

    // ── Test 3: múltiples frames en orden ────────────────────────────────
    //
    // Verifica que el total de bytes es correcto con varios frames de distintos tamaños.
    #[tokio::test]
    async fn test_consumer_multiples_frames() {
        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let done = Arc::new(AtomicBool::new(false));

        // 3 frames de tamaños distintos → total esperado: 100 + 200 + 300 = 600 bytes
        {
            let mut buf = buffer.lock().unwrap();
            buf.push_back(frame_vacio(0, 100));
            buf.push_back(frame_vacio(1, 200));
            buf.push_back(frame_vacio(2, 300));
        }
        done.store(true, Ordering::Release);

        let total = consume(buffer, done).await.unwrap();

        assert_eq!(total, 600);
    }

    // ── Test 4: estrés con múltiples workers ─────────────────────────────
    //
    // Peras y manzanas:
    //   Simulamos 8 inspectores (workers — trabajadores) sacando cajas de la misma
    //   cinta (buffer) al mismo tiempo. Verificamos que la suma total de bytes
    //   procesados es exacta sin importar cómo se repartieron las cajas.
    //
    // Detecta: deadlocks (bloqueos mutuos — dos hilos esperándose entre sí),
    //          pérdida de frames (cajas que desaparecen sin procesarse),
    //          data races (accesos simultáneos sin coordinación — imposibles en Rust
    //          pero el test verifica que la lógica también es correcta).
    #[tokio::test]
    async fn test_stress_multiples_workers() {
        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let done = Arc::new(AtomicBool::new(false));

        // 500 frames de 64 bytes cada uno → total esperado: 32_000 bytes
        {
            let mut buf = buffer.lock().unwrap();
            for i in 0..500 {
                buf.push_back(frame_vacio(i, 64));
            }
        }
        done.store(true, Ordering::Release);

        // JoinSet (conjunto de tareas — agrupa múltiples tasks async para esperar a todas)
        // Lanzamos 8 workers en paralelo contra el mismo buffer
        let mut workers = tokio::task::JoinSet::new();
        for _ in 0..8 {
            workers.spawn(consume(Arc::clone(&buffer), Arc::clone(&done)));
        }

        // Sumamos los bytes que procesó cada worker
        let mut total = 0usize;
        while let Some(res) = workers.join_next().await {
            total += res.unwrap().unwrap();
        }

        // El total debe ser exacto — ningún frame se pierde ni se procesa dos veces
        assert_eq!(total, 32_000);
    }

    // ── Test 5: proptest — total siempre exacto con tamaños aleatorios ────
    //
    // proptest genera automáticamente cientos de combinaciones de tamaños de frame
    // y verifica que consume() siempre devuelve el total correcto.
    // Detecta edge cases (casos límite) que los tests manuales nunca cubrirían:
    // frames de 1 byte, frames de tamaño máximo, listas de 1 o 50 frames, etc.
    //
    // Peras y manzanas: en vez de probar 3 cajas a mano, le pedimos a proptest
    // que pruebe miles de combinaciones aleatorias de cajas por nosotros.
    //
    // proptest! = macro (generador de código) que envuelve el test en un loop
    //             que corre con distintos valores generados aleatoriamente.
    proptest! {
        #[test]
        fn test_total_bytes_siempre_exacto(
            // sizes = lista aleatoria de entre 1 y 50 tamaños, cada uno entre 1 y 1024 bytes
            sizes in proptest::collection::vec(1usize..=1024, 1..=50)
        ) {
            // proptest no soporta async directamente — usamos block_on para ejecutar
            // la función async dentro del test síncrono (bloqueante — espera el resultado)
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let buffer = Arc::new(Mutex::new(VecDeque::new()));
                let done   = Arc::new(AtomicBool::new(false));

                // expected (esperado) = suma de todos los tamaños generados aleatoriamente
                let expected: usize = sizes.iter().sum();

                {
                    let mut buf = buffer.lock().unwrap();
                    for (i, &size) in sizes.iter().enumerate() {
                        buf.push_back(Frame {
                            index: i,
                            size,
                            data: Bytes::from(vec![0u8; size]),
                        });
                    }
                }
                done.store(true, Ordering::Release);

                let total = consume(buffer, done).await.unwrap();

                // prop_assert_eq! = como assert_eq! pero dentro de proptest —
                // si falla, muestra el caso mínimo que reproduce el error (shrinking)
                prop_assert_eq!(total, expected);
                Ok(())
            }).unwrap()
        }
    }
}
