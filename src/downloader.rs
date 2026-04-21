// ═══════════════════════════════════════════════════════════════
// ARCHIVO: downloader.rs — PRODUCTOR (el que trae la mercadería)
//
// Peras y manzanas:
//   Este archivo es el CAMIONERO de la fábrica.
//   Su trabajo es ir a buscar el video a internet (el depósito),
//   cortarlo en cajas de 64KB (frames — cuadros) y ponerlas
//   en la cinta transportadora (buffer — memoria intermedia)
//   para que el consumidor (pipeline.rs) las procese.
//
// FLUJO (orden de ejecución):
//  [VIENE DE] main.rs línea 81 → tokio::spawn lanza esta función
//      │
//      ├─► [PASO 1] pide el video a internet por HTTP
//      ├─► [PASO 2] verifica que el servidor respondió bien
//      ├─► [PASO 3] abre el stream (flujo de bytes que llegan de a poco)
//      ├─► [PASO 4] acumula bytes hasta juntar 64KB → arma un Frame (cuadro)
//      ├─► [PASO 5] pone el Frame en el buffer (cinta) compartido con pipeline.rs
//      └─► [PASO 6] cuando termina todo, activa el flag (bandera) `done` = true
//
//  [VA HACIA] buffer compartido → lo lee pipeline.rs
// ═══════════════════════════════════════════════════════════════

// use = importar (traer al archivo actual) código de otro lugar
use std::collections::VecDeque; // VecDeque (Vector Double-Ended Queue — cola de doble extremo):
// lista donde podés agregar/sacar por ambos extremos.
// El productor agrega por atrás, el consumidor saca por adelante.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex}; // Arc y Mutex son las herramientas de concurrencia (trabajo en paralelo) // AtomicBool y Ordering para el flag (bandera) `done`
use tracing::{debug, info};

use bytes::BytesMut; // BytesMut (Bytes Mutable — bytes modificables):
// como un Vec<u8> (lista de bytes) pero más eficiente
// para construir buffers (memorias intermedias) que luego
// se congelan (freeze — vuelven inmutables — no modificables)
use futures_util::StreamExt; // StreamExt (Stream Extensions — extensiones de stream — flujo):
// agrega el método .next() al stream HTTP para leer
// chunk (pedazo) por chunk (pedazo)

use crate::error::AppError; // crate = el proyecto actual (como un "paquete" de código)
// :: = operador de ruta, como / en una carpeta
use crate::frame::Frame; // importamos la estructura Frame

// ── TIPO: SharedBuffer (buffer compartido entre hilos) ────────────────────
//
// Peras y manzanas:
//   SharedBuffer es la CINTA TRANSPORTADORA de la fábrica.
//   Está hecha de tres capas:
//
//   1. VecDeque<Frame> — la cinta en sí: una cola de frames (cuadros)
//      donde el camionero (productor) pone cajas por atrás
//      y el inspector (consumidor) las saca por adelante.
//
//   2. Mutex<VecDeque<Frame>> — el CANDADO de la cinta:
//      garantiza que solo UN trabajador toca la cinta a la vez.
//      Si el camionero está poniendo una caja, el inspector espera.
//      Si el inspector está sacando una caja, el camionero espera.
//      Mutex = Mutual Exclusion (exclusión mutua — solo uno a la vez).
//
//   3. Arc<Mutex<VecDeque<Frame>>> — la DIRECCIÓN DE LA CINTA:
//      permite que tanto el camionero como el inspector tengan
//      una referencia (puntero — dirección de memoria) a la MISMA cinta.
//      Arc = Atomic Reference Counted (contador atómico de referencias —
//      cuenta cuántos "dueños" apuntan al mismo dato; cuando llega a 0,
//      libera la memoria automáticamente).
//
// `pub type` = definir un alias (apodo, nombre alternativo) para un tipo largo.
// Así escribimos SharedBuffer en vez de Arc<Mutex<VecDeque<Frame>>> en todos lados.
pub type SharedBuffer = Arc<Mutex<VecDeque<Frame>>>;

// ── TIPO: SharedDone (bandera compartida de "terminé") ────────────────────
//
// Peras y manzanas:
//   Es la LUZ DE "OCUPADO" del baño de la oficina.
//   Cuando el camionero termina de traer toda la mercadería,
//   apaga la luz (done = false → true) para que el inspector sepa
//   que no van a llegar más cajas.
//
// Por qué AtomicBool (booleano atómico — verdadero/falso seguro entre hilos)
// y no Arc<Mutex<bool>> (la misma idea pero con candado)?
//   Para un simple verdadero/falso, el candado (Mutex) es demasiado pesado —
//   suspende el hilo (thread — unidad de ejecución paralela) si está ocupado.
//   AtomicBool hace lo mismo pero sin suspender: la CPU garantiza que la
//   operación de leer/escribir el bool es atómica (indivisible — no puede
//   interrumpirse a la mitad), sin necesitar un candado.
pub type SharedDone = Arc<AtomicBool>;

// ── [ENTRADA DESDE main.rs] ───────────────────────────────────────────────
// Esta función es llamada por tokio::spawn (lanzador de tareas concurrentes)
// desde main.rs línea 81. Corre en paralelo con consume() de pipeline.rs.
//
// Parámetros (datos de entrada de la función):
//   url    → &str (&str = referencia a texto — dirección web del video)
//   buffer → SharedBuffer (la cinta transportadora compartida)
//   done   → SharedDone (la bandera de "terminé")
//
// Retorna (devuelve):
//   Result<(), AppError>
//   Result = tipo que puede ser Ok (éxito) o Err (error — falla).
//   ()     = "nada" (unidad vacía — como void en C/C++).
//   Si todo sale bien: Ok(())
//   Si algo falla:     Err(AppError::Http(...)) o Err(AppError::LockPoisoned) etc.
pub async fn stream_into_buffer(
    url: &str,
    frame_size: usize,
    buffer: SharedBuffer,
    done: SharedDone,
) -> Result<(), AppError> {
    // ── [PASO 1] Pedir el video a internet ────────────────────────────
    //
    // reqwest::get(url) envía una petición HTTP GET (solicitud de descarga)
    // a la URL (dirección web) del video.
    // HTTP GET = "dame el archivo que está en esta dirección".
    //
    // .await = esperar el resultado sin bloquear el hilo (thread — unidad de ejecución).
    // Peras y manzanas: es como pedir una pizza por teléfono y seguir viendo tele
    // mientras esperás — no te quedás parado en la puerta esperando.
    //
    // ? = operador de propagación de errores (si falla, sale de la función con Err).
    // Si la URL no existe o no hay internet: devuelve Err(AppError::Http(...)).
    info!("Downloader: Fetching URL - {}", url);
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .send()
        .await?;
    info!("Downloader: Response received with status: {}", response.status());

    // ── [PASO 2] Verificar que el servidor respondió bien ─────────────
    //
    // El servidor HTTP (computadora que tiene el video) responde con un código:
    //   2xx (ej: 200 OK)       → todo bien, el video viene
    //   4xx (ej: 404 Not Found — no encontrado) → URL (dirección) equivocada
    //   5xx (ej: 500 Internal Server Error — error interno del servidor) → el servidor falló
    //
    // is_success() devuelve true (verdadero) solo si el código es 2xx.
    // Si no es éxito, retornamos UnexpectedEnd (fin inesperado)
    // en vez de intentar procesar una página de error como si fuera video.
    if !response.status().is_success() {
        return Err(AppError::UnexpectedEnd);
    }

    // ── [PASO 3] Abrir el stream (flujo) de bytes ─────────────────────
    //
    // bytes_stream() convierte la respuesta en un stream (flujo continuo de datos).
    // NO descarga todo el video a memoria (RAM — memoria de acceso rápido) de golpe.
    //
    // Peras y manzanas:
    //   Sin stream (flujo): esperás que llegue el camión COMPLETO (158 MB)
    //   antes de empezar a trabajar. El depósito (RAM) se llena.
    //   Con stream (flujo): las cajas van llegando de a una y las procesás
    //   al vuelo — el depósito siempre tiene solo unas pocas cajas.
    let mut stream = response.bytes_stream();

    // accumulator (acumulador): buffer (memoria temporal) donde juntamos
    // bytes de varios chunks (pedazos HTTP) hasta completar un Frame (cuadro) de 64KB.
    //
    // Peras y manzanas:
    //   Los chunks (pedazos) que llegan de la red tienen tamaño variable (~8KB cada uno).
    //   El accumulator es como una balanza: seguimos agregando chunks hasta que
    //   pese 64KB — recién ahí armamos un Frame y lo mandamos a la cinta.
    //
    // BytesMut = buffer mutable (modificable — podemos agregarle bytes).
    // .new() = crear uno vacío, sin bytes todavía.
    let mut accumulator = BytesMut::new();

    // frame_index (índice del frame — cuadro): contador del número de frame actual.
    // Empieza en 0 y sube de 1 en 1 cada vez que creamos un Frame.
    // usize = número entero positivo (sin signo).
    let mut frame_index = 0usize;

    // ── [PASO 3 continúa] Leer chunk (pedazo) por chunk del stream ────
    //
    // while let Some(chunk) = stream.next().await:
    //   stream.next() pide el siguiente chunk (pedazo de datos) del stream (flujo).
    //   .await = esperar sin bloquear (ver explicación en PASO 1).
    //   Devuelve Some(chunk) si llegó un pedazo, o None si el stream terminó.
    //   while let = mientras siga llegando Some(chunk), ejecutar el bloque.
    while let Some(chunk) = stream.next().await {
        // chunk acá es Result<Bytes, reqwest::Error>
        // (puede ser un pedazo de bytes OK, o un error de red).
        // ? = si hay error de red en este chunk (pedazo), salir con Err.
        let chunk = chunk?;

        // Copiamos los bytes del chunk al acumulador (balanza).
        // extend_from_slice (extender desde slice — vista de bytes):
        //   &chunk convierte el chunk en un slice (vista — referencia a los bytes)
        //   y los agrega al final del acumulador.
        // Necesitamos copiar porque el chunk tiene su propio lifetime (tiempo de vida —
        // cuánto tiempo existe el dato en memoria) y desaparecería al final del loop.
        accumulator.extend_from_slice(&chunk);

        // ── [PASO 4] Cuando el acumulador tiene n KB: crear un Frame ──
        //
        // Usamos while (mientras) y no if (si) porque un chunk (pedazo)
        // podría llenar MÁS de un frame — si el chunk es grande.
        while accumulator.len() >= frame_size {
            // split_to(n) corta y devuelve los primeros n bytes del acumulador.
            // Lo que sobra queda en accumulator para el siguiente frame.
            // O(1) = tiempo constante — no importa cuántos bytes tenga, es igual de rápido.
            // Peras y manzanas: es como cortar los primeros 64 páginas del libro
            // sin tocar el resto.
            let frame_bytes = accumulator.split_to(frame_size);

            // Armamos el Frame (cuadro — la caja de la cinta) con sus tres campos:
            let frame = Frame {
                index: frame_index, // número de orden (0, 1, 2, ...)
                size: frame_size,   // peso de la caja: siempre frame_size acá (el último puede ser menor)

                // freeze() (congelar) convierte BytesMut (mutable — modificable)
                // en Bytes (inmutable — solo lectura).
                // Una vez congelado, múltiples hilos (threads — unidades de ejecución)
                // pueden leerlo al mismo tiempo sin riesgo de que uno lo modifique.
                // Peras y manzanas: sellás la caja — ya nadie puede agregarle cosas adentro.
                data: frame_bytes.freeze(),
            };

            // ── [PASO 5] Poner el Frame en el buffer (cinta) compartido ──
            //
            // .lock() toma el Mutex (candado de exclusión mutua):
            //   - Si el candado está libre: lo toma y devuelve Ok(MutexGuard)
            //   - Si el candado lo tiene otro hilo (thread): ESPERA hasta que se libere
            //   - Si el candado está envenenado (poisoned — un hilo colapsó con el lock):
            //     devuelve Err(PoisonError)
            //
            // MutexGuard (guardia del candado): mientras existe, tenés el candado.
            // Cuando el MutexGuard sale de scope (alcance — fin del bloque de código),
            // el candado se libera automáticamente. No hay unlock() explícito.
            // Peras y manzanas: la llave del baño vuelve al gancho sola cuando salís.
            //
            // .map_err(|_| AppError::LockPoisoned) convierte el error raro PoisonError
            // en nuestro error limpio AppError::LockPoisoned.
            //
            // ? = si hay error, salir con Err(AppError::LockPoisoned).
            //
            // .push_back(frame) agrega el Frame al FONDO de la VecDeque (cola).
            // El consumidor (pipeline.rs) lo sacará por el FRENTE con pop_front().
            // ──► este Frame ya está disponible para que pipeline.rs lo consuma
            buffer
                .lock()
                .map_err(|_| AppError::LockPoisoned)?
                .push_back(frame);

            frame_index += 1; // += 1 = sumar 1 al contador (frame_index = frame_index + 1)
            debug!("Produced frame #{} ({} bytes)", frame_index, frame_size);
        }
    }

    // ── Frame parcial (incompleto) final ──────────────────────────────
    //
    // Cuando el stream (flujo) termina, el acumulador puede tener bytes
    // sobrantes que no llegaron a completar 64KB.
    // Peras y manzanas: la última caja de la cinta viene a medio llenar —
    // la mandamos igual en vez de tirar esos bytes.
    //
    // .is_empty() devuelve true (verdadero) si el acumulador no tiene nada.
    // ! = negación lógica (NOT — no): !is_empty() = "si NO está vacío"
    if !accumulator.is_empty() {
        let remaining = accumulator.len(); // len() = length (largo — cantidad de bytes)
        let frame = Frame {
            index: frame_index,
            size: remaining,            // tamaño real — menor que FRAME_SIZE (64KB)
            data: accumulator.freeze(), // freeze() = congelar (volver inmutable — solo lectura)
        };
        buffer
            .lock()
            .map_err(|_| AppError::LockPoisoned)?
            .push_back(frame);
        frame_index += 1;
        debug!(
            "Produced partial frame #{} ({} bytes)",
            frame_index, remaining
        );
    }

    // ── [PASO 6 — SALIDA] Activar la bandera de "terminé" ────────────
    //
    // done.store(true, Ordering::Release):
    //   .store(true, ...) = guardar el valor true (verdadero) en el AtomicBool (bandera atómica).
    //
    //   Ordering::Release (orden de liberación):
    //   Peras y manzanas: es como firmar el remito del camión.
    //   Garantiza que TODOS los frames que pusimos en el buffer ANTES de esta línea
    //   sean visibles para el consumidor (pipeline.rs) cuando vea done=true.
    //   Sin Ordering::Release, el procesador podría reordenar instrucciones
    //   y el consumidor podría ver done=true ANTES de ver los últimos frames —
    //   saldría y perdería datos.
    //
    // ──► pipeline.rs verá done=true y saldrá de su loop (bucle) cuando el buffer esté vacío
    done.store(true, Ordering::Release);

    info!(
        "Downloader finished. Total frames produced: {}",
        frame_index
    );
    Ok(()) // Ok(()) = éxito, sin valor de retorno (solo informamos que terminó bien)
    // [FIN DE downloader.rs] ──► el control vuelve a main.rs : tokio::join!
}

// ── Tests con wiremock (servidor HTTP falso) ───────────────────────────────
//
// wiremock levanta un servidor HTTP real en un puerto local aleatorio.
// Permite testear stream_into_buffer sin depender de internet.
// Peras y manzanas: en vez de ir al depósito real (internet), usamos
// un depósito de cartón (mock server) que devuelve lo que nosotros decidimos.
//
// Correr: cargo test
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ── Test 1: descarga exitosa ──────────────────────────────────────────
    //
    // El mock server devuelve exactamente 3 frames completos de 64 KB.
    // Verificamos que el buffer recibe los 3 frames y done queda en true.
    #[tokio::test]
    async fn test_download_exitoso() {
        // MockServer (servidor simulado) levanta un servidor HTTP local en un puerto aleatorio.
        // Se destruye automáticamente al salir del test.
        let server = MockServer::start().await;

        // 3 frames × 65536 bytes = 196608 bytes en total
        let cuerpo = vec![0u8; 65536 * 3];

        // Mock::given(...): definir qué petición interceptar y qué responder.
        // method("GET") = interceptar solo peticiones GET (descarga).
        // respond_with: devolver 200 OK con el cuerpo de bytes.
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(cuerpo))
            .mount(&server)
            .await;

        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let done = Arc::new(AtomicBool::new(false));

        // server.uri() devuelve "http://127.0.0.1:<puerto>" del servidor local
        stream_into_buffer(&server.uri(), 65536, Arc::clone(&buffer), Arc::clone(&done))
            .await
            .unwrap();

        // El productor debe haber puesto exactamente 3 frames en el buffer
        assert_eq!(buffer.lock().unwrap().len(), 3);
        // Y debe haber activado la bandera de "terminé"
        assert!(done.load(Ordering::Acquire));
    }

    // ── Test 2: servidor responde 404 ────────────────────────────────────
    //
    // El mock server devuelve 404 Not Found (no encontrado).
    // Verificamos que stream_into_buffer retorna Err en vez de panic.
    // Peras y manzanas: el camionero llega al depósito y no existe — debe
    // volver con un reporte de error, no chocar el camión.
    #[tokio::test]
    async fn test_download_error_404() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404)) // 404 = Not Found (no encontrado)
            .mount(&server)
            .await;

        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let done = Arc::new(AtomicBool::new(false));

        let resultado =
            stream_into_buffer(&server.uri(), 65536, Arc::clone(&buffer), Arc::clone(&done)).await;

        // is_err() = verdadero si el Result es Err — el error fue capturado, no hubo panic
        assert!(resultado.is_err());
        // El buffer debe seguir vacío — no se procesó nada
        assert!(buffer.lock().unwrap().is_empty());
    }
}
