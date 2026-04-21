// ═══════════════════════════════════════════════════════════════
// ARCHIVO: logger.rs — sistema de logging (registro de eventos)
//
// Peras y manzanas:
//   En vez de usar println! para imprimir mensajes en pantalla,
//   usamos tracing (rastreo — sistema de registro estructurado).
//   La diferencia es como entre escribir en un bloc de notas (println!)
//   vs llevar un libro contable oficial (tracing):
//     - El libro tiene timestamp (marca de tiempo — cuándo ocurrió)
//     - Tiene nivel de importancia (info, warn, error, debug)
//     - Se guarda en archivos rotativos (un archivo por día, se borran los viejos)
//     - Y también se muestra en pantalla al mismo tiempo
//
// Flujo:
//   logger_init(&cfg) → configura tracing → devuelve WorkerGuard
//   WorkerGuard debe vivir mientras dure el programa (se guarda en _guard en main.rs)
// ═══════════════════════════════════════════════════════════════

use std::io; // io (Input/Output — Entrada/Salida): para escribir a stdout (pantalla)

// tracing::Level: niveles de log (registro) disponibles
// ERROR < WARN < INFO < DEBUG < TRACE (de menos a más detallado)
use tracing::Level;

// tracing_appender: escribe logs a archivos con rotación (rotation — archivo nuevo cada día)
//   non_blocking: escribe en un hilo (thread) separado para no bloquear el programa
//   WorkerGuard: guardia — si se destruye, el escritor de logs se detiene
//   RollingFileAppender: escritor con rotación de archivos
//   Rotation: frecuencia de rotación (DAILY = diario, HOURLY = cada hora, NEVER = nunca)
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};

// tracing_subscriber: conecta el sistema tracing con sus destinos de salida
//   EnvFilter: permite controlar el nivel de log con la variable de entorno RUST_LOG
//              (ej: RUST_LOG=debug cargo run muestra todos los mensajes de debug)
//   fmt: formateador — cómo se ve el log en pantalla o en archivo
//   SubscriberExt: trait (rasgo — capacidad) para encadenar capas de logging
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

use crate::configuration::Configuration; // leemos log_dir y level de la configuración

// logger_init (inicializar logger — registrador):
//   Configura el sistema de logging del programa.
//   Devuelve un WorkerGuard (guardia del escritor) que DEBE mantenerse vivo
//   mientras el programa corra. Si se destruye (drop — liberar), los logs
//   dejan de escribirse al archivo.
//
//   Peras y manzanas:
//     WorkerGuard es como el conserje de la oficina — mientras esté en su puesto,
//     los mensajes llegan al archivo. Si se va a casa (drop), nadie los recibe.
//
//   Por eso en main.rs lo guardamos en _guard (el _ le dice a Rust
//   "esta variable no se usa directamente pero no la tires todavía").
pub fn logger_init(cfg: &Configuration) -> WorkerGuard {
    // EnvFilter (filtro por variable de entorno):
    // Lee el nivel de log desde la variable de entorno RUST_LOG si existe,
    // o usa el nivel del config.toml como valor por defecto.
    //
    // Ejemplo: RUST_LOG=debug cargo run → muestra todos los mensajes debug
    //          Sin RUST_LOG → usa cfg.logging.level ("info" por defecto)
    let default_level: Level = cfg
        .logging
        .level
        .parse()
        .unwrap_or(Level::INFO); // unwrap_or = si falla el parse (análisis), usar INFO

    let filter = EnvFilter::builder()
        .with_default_directive(default_level.into()) // directive (directiva — regla de filtrado)
        .from_env_lossy(); // lossy (con pérdida) = si RUST_LOG tiene valores inválidos, los ignora

    // RollingFileAppender (escritor de archivo con rotación):
    //   Crea un archivo nuevo cada día en cfg.logging.log_dir.
    //   Formato del nombre: "stream-pipe.2026-04-21.log"
    //   Guarda los últimos 14 días de logs y borra los más viejos.
    //
    //   Peras y manzanas:
    //     Es como una carpeta de "archivo de correspondencia" donde guardás
    //     las cartas de los últimos 14 días y tirás las más viejas.
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY) // un archivo nuevo por día
        .max_log_files(14) // mantener los últimos 14 días
        .filename_prefix("stream-pipe") // nombre base del archivo de log
        .filename_suffix("log") // extensión del archivo
        .build(&cfg.logging.log_dir) // carpeta donde se guardan los archivos
        .expect("No se pudo configurar el escritor de logs — verificá que log_dir existe");

    // non_blocking (sin bloqueo):
    //   Convierte el escritor de archivo en uno asíncrono (no bloquea el hilo).
    //   Los logs se escriben en un buffer (memoria temporal) y un hilo (thread)
    //   separado los vuelca al disco en segundo plano.
    //
    //   Peras y manzanas:
    //     En vez de que el programa se detenga cada vez que escribe un log
    //     (como si tuvieras que esperar que el cartero pase antes de seguir trabajando),
    //     dejás los mensajes en un buzón y el cartero los recoge solo.
    //
    //   guard = WorkerGuard — el "cartero" — debe vivir mientras el programa corra.
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Construimos el subscriber (suscriptor — receptor de eventos de log) con dos capas (layers):
    //
    //   Capa 1: fmt::Layer con writer io::stdout → imprime en pantalla (terminal)
    //   Capa 2: fmt::Layer con writer non_blocking → escribe en archivo de log en disco
    //
    //   Peras y manzanas:
    //     Es como un megáfono que al mismo tiempo habla en voz alta (pantalla)
    //     y graba en un cassette (archivo en disco).
    let subscriber = tracing_subscriber::registry() // registry (registro) = núcleo del sistema
        .with(filter) // aplicar el filtro de nivel a ambas capas
        .with(fmt::Layer::new().with_writer(io::stdout)) // capa de pantalla
        .with(fmt::Layer::new().with_writer(non_blocking)); // capa de archivo

    // set_global_default (establecer el suscriptor global por defecto):
    //   A partir de acá, todo tracing::info!(), tracing::warn!(), etc.
    //   en cualquier parte del programa usará este subscriber.
    tracing::subscriber::set_global_default(subscriber)
        .expect("No se pudo inicializar el sistema de logging");

    guard // devolvemos el guard para que main.rs lo mantenga vivo
}
