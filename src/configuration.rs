// ═══════════════════════════════════════════════════════════════
// ARCHIVO: configuration.rs — configuración del programa
//
// Peras y manzanas:
//   Este archivo es el "tablero de control" del programa.
//   En vez de hardcodear (escribir fijo en el código) la URL del video,
//   el tamaño de los frames o dónde guardar los logs, los leemos
//   de un archivo config.toml (archivo de configuración en formato TOML).
//   Así podés cambiar el comportamiento sin recompilar el programa.
//
// Flujo:
//   config.toml (en disco) → Configuration::new() → struct Configuration
//   → se pasa a stream_into_buffer() y consume() como parámetro
// ═══════════════════════════════════════════════════════════════

// Config: librería (biblioteca de código) que lee archivos de configuración.
// File: representa un archivo de configuración en disco.
use config::{Config, File};

// Deserialize (deserializar — convertir texto a struct — estructura de datos):
// permite que serde (framework — estructura base de serialización) convierta
// el config.toml automáticamente a nuestra struct Configuration.
use serde::Deserialize;

// ── STRUCT (estructura de datos) principal de configuración ───────────────
//
// #[derive(Debug, Deserialize)]:
//   Debug       → permite imprimir la configuración con {:?} para verificar que cargó bien
//   Deserialize → permite convertir el config.toml automáticamente a esta struct
//
// Peras y manzanas:
//   Es como un formulario con campos. Cuando el programa arranca,
//   rellena el formulario leyendo el config.toml.
#[derive(Debug, Deserialize)]
pub struct Configuration {
    // [video] en config.toml — configuración de la fuente de datos
    pub video: VideoConfig,

    // [pipeline] en config.toml — configuración del procesamiento
    pub pipeline: PipelineConfig,

    // [logging] en config.toml — configuración del sistema de registros
    pub logging: LoggingConfig,
}

// ── Configuración del video (fuente de datos) ─────────────────────────────
#[derive(Debug, Deserialize)]
pub struct VideoConfig {
    // URL (dirección web) del video a procesar.
    // Ejemplo: "http://commondatastorage.googleapis.com/.../BigBuckBunny.mp4"
    pub url: String,

    // Tamaño de cada frame (cuadro) simulado en bytes (unidades mínimas de datos).
    // Por defecto: 65536 (64 KB — kilobytes).
    // Peras y manzanas: el tamaño de cada "caja" en la cinta transportadora.
    pub frame_size: usize,
}

// ── Configuración del pipeline (tubería de procesamiento) ─────────────────
#[derive(Debug, Deserialize)]
pub struct PipelineConfig {
    // Cantidad de workers (trabajadores — consumidores) procesando frames en paralelo.
    // 1 = un solo consumidor (configuración actual del proyecto).
    // N = múltiples consumidores (para simular múltiples usuarios simultáneos).
    pub workers: usize,
}

// ── Configuración del sistema de logs (registros de eventos) ──────────────
#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    // Nivel de detalle de los logs (registros).
    // Opciones de menor a mayor detalle:
    //   "error" → solo errores críticos
    //   "warn"  → advertencias y errores
    //   "info"  → información general (recomendado en producción)
    //   "debug" → información detallada (útil mientras desarrollás)
    //   "trace" → todo (muy verboso — detallado, para depuración profunda)
    pub level: String,

    // Directorio (carpeta) donde se guardan los archivos de log.
    // Ejemplo: "/var/log/stream-pipe" en Linux, "logs/" en desarrollo local.
    pub log_dir: String,
}

// ── impl (implementación — métodos propios del tipo) de Configuration ──────
impl Configuration {
    // new() crea una Configuration leyendo el config.toml.
    //
    // Busca el archivo en dos lugares (en orden de prioridad):
    //   1. /var/lib/stream-pipe/config.toml → producción (servidor instalado)
    //   2. config.toml en la raíz del proyecto → desarrollo local
    //
    // Peras y manzanas:
    //   Es como buscar el manual de instrucciones primero en el cajón del trabajo
    //   y si no está ahí, en el cajón de casa.
    pub fn new() -> Self {
        let cfg = Config::builder()
            // 1. Intentamos cargar el archivo de producción (opcional)
            .add_source(File::with_name("/var/lib/stream-pipe/config.toml").required(false))
            // 2. Intentamos cargar el archivo local. Si no existe ninguno, esto fallará
            // de forma clara en el .build() o en el try_deserialize.
            .add_source(File::with_name("config.toml").required(false))
            .build()
            .map_err(|e| format!("Error al construir configuración: {}", e))
            .expect("No se pudo encontrar ningún archivo config.toml válido");

        cfg.try_deserialize()
            .expect("El config.toml no se encontró o le faltan campos obligatorios (video, pipeline, logging)")
    }
}

// ── Tests (pruebas automatizadas) ─────────────────────────────────────────
//
// #[cfg(test)] = este bloque solo se compila cuando corrés `cargo test`.
// No afecta el binario (ejecutable) de producción.
#[cfg(test)]
mod tests {
    use super::*;
    use config::Config;

    // Función auxiliar (helper — ayudante): convierte texto TOML directamente
    // a Configuration sin leer un archivo del disco — ideal para tests rápidos.
    fn parse_toml(toml: &str) -> Configuration {
        Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    }

    #[test]
    fn test_configuracion_completa() {
        let cfg = parse_toml(
            r#"
            [video]
            url = "http://example.com/video.mp4"
            frame_size = 65536

            [pipeline]
            workers = 1

            [logging]
            level = "info"
            log_dir = "/var/log/stream-pipe"
        "#,
        );

        assert_eq!(cfg.video.url, "http://example.com/video.mp4");
        assert_eq!(cfg.video.frame_size, 65536);
        assert_eq!(cfg.pipeline.workers, 1);
        assert_eq!(cfg.logging.level, "info");
    }

    #[test]
    fn test_frame_size_personalizado() {
        let cfg = parse_toml(
            r#"
            [video]
            url = "http://example.com/video.mp4"
            frame_size = 32768

            [pipeline]
            workers = 2

            [logging]
            level = "debug"
            log_dir = "logs/"
        "#,
        );

        assert_eq!(cfg.video.frame_size, 32768); // 32 KB — mitad del tamaño por defecto
        assert_eq!(cfg.pipeline.workers, 2);
    }
}
