use thiserror::Error;

// ═══════════════════════════════════════════════════════════════
// ARCHIVO: error.rs — tipos de error (falla) del programa
//
// Peras y manzanas:
//   Imaginate que trabajás en una fábrica y algo sale mal.
//   En vez de que cada empleado invente su propio formulario de queja,
//   hay UN solo formulario oficial con casillas:
//     □ Falla de camión (red caída)
//     □ Falla de candado (lock envenenado)
//     □ Falla de suministro (stream cortado)
//   Eso es AppError — el formulario único de errores de este programa.
// ═══════════════════════════════════════════════════════════════

// #[derive(Debug, Error)] le pide a Rust que genere automáticamente:
//   - Debug  → capacidad de imprimir el error con {:?} para ver sus detalles internos
//   - Error  → que este tipo (estructura de datos) sea reconocido como un error oficial de Rust
// Sin esto tendríamos que escribir ese código a mano — mucho trabajo repetitivo.
#[derive(Debug, Error)]
pub enum AppError {
    // ── Variante 1: error de red (HTTP) ───────────────────────────────
    //
    // HTTP (HyperText Transfer Protocol — Protocolo de Transferencia de Hipertexto)
    // es el idioma que usan las computadoras para pedirse archivos por internet.
    // Cuando algo falla en esa comunicación (red caída, URL — dirección web inválida,
    // servidor apagado), reqwest (la librería — biblioteca de código que usamos
    // para hacer peticiones HTTP) produce un reqwest::Error (error de reqwest).
    //
    // #[from] le dice a Rust:
    //   "si en algún lugar del código aparece un reqwest::Error,
    //    conviértelo automáticamente a AppError::Http sin que yo
    //    tenga que escribir la conversión a mano"
    //
    // Esto es lo que hace que el operador `?` (propagador de errores —
    // atajo que dice "si esto falla, devolvé el error y salí de la función")
    // funcione sin código extra.
    //
    // Ejemplo de uso:
    //   reqwest::get(url).await?   ← el ? convierte reqwest::Error → AppError::Http
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    // ── Variante 2: candado envenenado ────────────────────────────────
    //
    // Peras y manzanas:
    //   Imaginate que el candado del baño de la oficina (Mutex —
    //   candado de exclusión mutua, que solo deja entrar a uno a la vez)
    //   se traba porque alguien se desmayó adentro con el candado puesto.
    //   Nadie más puede entrar — el candado quedó "envenenado".
    //
    // En código: si un hilo (thread — unidad de ejecución paralela) hace
    // panic (colapso inesperado del programa) mientras tiene el Mutex tomado,
    // el Mutex queda en estado "envenenado" (poisoned).
    // Cuando otro hilo intenta tomarlo, lock() devuelve Err(PoisonError)
    // en vez de Ok(MutexGuard — guardia del candado).
    //
    // En vez de dejar que ese PoisonError raro salga al exterior,
    // lo convertimos a nuestra variante limpia LockPoisoned.
    #[error("Pipeline poisoned: lock was poisoned")]
    LockPoisoned,

    // ── Variante 3: stream cortado ────────────────────────────────────
    //
    // Stream (flujo continuo de datos) es como una canilla de agua abierta:
    // los bytes van llegando de a poco desde internet.
    // Si la conexión se corta a mitad de la descarga, o el servidor responde
    // con un error (404 — no encontrado, 500 — error interno del servidor),
    // el stream no tiene los datos que esperábamos.
    // UnexpectedEnd (fin inesperado) señaliza exactamente esa situación.
    #[error("Stream ended unexpectedly")]
    UnexpectedEnd,
}
