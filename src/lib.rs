// lib.rs — punto de entrada de la librería (library crate)
//
// Peras y manzanas:
//   Un proyecto Rust puede ser binario (ejecutable — tiene main.rs) o
//   librería (biblioteca — tiene lib.rs) o AMBOS al mismo tiempo.
//   Al agregar lib.rs, los benchmarks (benches/) y tests externos pueden
//   importar módulos del proyecto con `use stream_pipe::pipeline::consume`.
//   El binario (main.rs) sigue funcionando igual que antes.

pub mod configuration;
pub mod downloader;
pub mod error;
pub mod frame;
pub mod logger;
pub mod pipeline;
