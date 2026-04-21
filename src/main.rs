// ═══════════════════════════════════════════════════════════════
// ARCHIVO: main.rs — punto de entrada del programa
//
// Peras y manzanas:
//   Este archivo es el JEFE DE LA FÁBRICA.
//   No hace el trabajo pesado — contrata y coordina a dos empleados:
//     1. El CAMIONERO (downloader.rs): trae el video de internet
//        y lo corta en cajas (frames — cuadros) en la cinta (buffer).
//     2. El INSPECTOR (pipeline.rs): saca cajas de la cinta
//        y procesa su contenido.
//   El jefe arranca a ambos AL MISMO TIEMPO y espera que terminen.
//
// FLUJO GENERAL (orden de ejecución del programa):
//
//  [ENTRADA] → main() arranca
//      │
//      ├─► [PASO 1] crea la cinta transportadora (buffer compartido)
//      ├─► [PASO 2] crea la bandera "terminé" (done — flag atómico)
//      ├─► [PASO 3] lanza el CAMIONERO  ──► downloader.rs
//      ├─► [PASO 4] lanza el INSPECTOR  ──► pipeline.rs
//      ├─► [PASO 5] espera que AMBOS terminen (tokio::join!)
//      └─► [PASO 6] imprime total de bytes y termina
//  [SALIDA]
// ═══════════════════════════════════════════════════════════════

// mod = módulo (unidad de código separada en su propio archivo).
// Esto le dice a Rust "existe un archivo downloader.rs, error.rs, etc."
// y los incluye en el programa.
mod configuration; // el tablero de control — lee el archivo config.toml
mod downloader; // el camionero — descarga el video de internet
mod error; // el formulario de incidentes — tipos de error del programa
mod frame; // la definición de "caja" — qué es un Frame (cuadro de video)
mod logger; // el sistema de registros — escribe en pantalla y archivo
mod pipeline; // el inspector — procesa los frames (cuadros)

// use = importar (traer al alcance actual) nombres de tipos y funciones
// para no tener que escribir el camino completo cada vez.
use std::collections::VecDeque; // VecDeque (Vector Double-Ended Queue — cola de doble extremo):
// lista donde el camionero agrega por atrás y el inspector saca por adelante
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex}; // Arc (puntero compartido atómico) y Mutex (candado de exclusión mutua) // AtomicBool (booleano — verdadero/falso — atómico — seguro entre hilos)
use crate::configuration::Configuration;
use crate::downloader::{SharedBuffer, SharedDone, stream_into_buffer}; // función y tipos del camionero
use crate::logger::logger_init;
use crate::pipeline::consume; // función del inspector

// #[tokio::main] es un atributo (macro — generador de código) que transforma
// la función main() en una función async (asíncrona — puede usar .await).
#[tokio::main]
async fn main() -> Result<(), error::AppError> {
    // ── [PASO 0] Cargar configuración e inicializar logger ───────────
    //
    // Primero leemos el config.toml (archivo de configuración).
    let cfg = Configuration::new();

    // Iniciamos el sistema de logs (registros).
    // El _guard debe vivir mientras el programa corra para que los logs
    // lleguen al archivo antes de que el programa cierre.
    let _guard = logger_init(&cfg);

    tracing::info!("Pipeline arrancando — workers: {}", cfg.pipeline.workers);

    // ── [PASO 1] Crear la cinta transportadora compartida ─────────────
    let buffer: SharedBuffer = Arc::new(Mutex::new(VecDeque::new()));

    // ── [PASO 2] Crear la bandera de "terminé" ────────────────────────
    let done: SharedDone = Arc::new(AtomicBool::new(false));

    // ── [PASO 3] Preparar y lanzar el CAMIONERO ───────────────────────
    let buffer_prod = Arc::clone(&buffer);
    let done_prod = Arc::clone(&done);

    // Clonamos la URL de la configuración para pasarla al hilo.
    let video_url = cfg.video.url.clone();
    let frame_size = cfg.video.frame_size;

    let producer = tokio::spawn(async move {
        // ──► continúa en downloader.rs : fn stream_into_buffer()
        stream_into_buffer(&video_url, frame_size, buffer_prod, done_prod).await
    });

    // ── [PASO 4] Lanzar el INSPECTOR (o varios) ───────────────────────
    let workers = cfg.pipeline.workers;
    let mut consumers = tokio::task::JoinSet::new();

    for i in 0..workers {
        let buffer_cons = Arc::clone(&buffer);
        let done_cons = Arc::clone(&done);
        consumers.spawn(async move {
            tracing::info!("Worker #{} started", i + 1);
            // ──► continúa en pipeline.rs : fn consume()
            consume(buffer_cons, done_cons).await
        });
    }

    // ── [PASO 5] Esperar que AMBOS terminen ───────────────────────────
    // Esperamos al productor primero
    let producer_result = producer.await;

    // Luego esperamos a todos los consumidores y sumamos sus totales
    let mut total = 0;
    while let Some(res) = consumers.join_next().await {
        total += res.expect("consumer task panicked")?;
    }

    // ── [PASO 6 — SALIDA] Verificar errores y terminar ────────────────
    producer_result.expect("producer task panicked")?;

    // [FIN DEL PROGRAMA] — todo salió bien
    tracing::info!("Pipeline complete. Total bytes processed: {}", total);

    // Ok(()) = retornar éxito sin valor (el programa terminó correctamente).
    Ok(())
}
