// ═══════════════════════════════════════════════════════════════
// ARCHIVO: benches/pipeline_bench.rs — benchmarks de rendimiento
//
// Peras y manzanas:
//   Un benchmark (medición de rendimiento) es como un cronómetro profesional
//   para el código: no solo mide una vez, sino que repite la misma operación
//   miles de veces y calcula la media, la varianza y detecta si el código
//   se volvió más lento entre versiones (regresión de rendimiento).
//
// Herramienta: criterion (criterio — librería de benchmarking estadístico).
//   A diferencia de medir tiempo con Instant::now(), criterion:
//   - calienta el CPU (warm-up — evita que la primera medición sea lenta)
//   - detecta outliers (valores atípicos — mediciones anómalas)
//   - muestra percentil 95/99 además de la media
//
// Correr todos los benchmarks:
//   cargo bench
//
// Correr un benchmark específico:
//   cargo bench bench_consume_100_frames
//
// Los resultados se guardan en: target/criterion/<nombre>/report/index.html
// ═══════════════════════════════════════════════════════════════

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use stream_pipe::frame::Frame;
use stream_pipe::pipeline::consume;

// helper (función auxiliar): crea un buffer (cinta) con N frames de tamaño fijo.
// Devuelve (SharedBuffer, SharedDone) listos para pasar a consume().
fn preparar_buffer(n_frames: usize, frame_size: usize) -> (Arc<Mutex<VecDeque<Frame>>>, Arc<AtomicBool>) {
    let buffer = Arc::new(Mutex::new(VecDeque::new()));
    let done   = Arc::new(AtomicBool::new(false));

    {
        let mut buf = buffer.lock().unwrap();
        for i in 0..n_frames {
            buf.push_back(Frame {
                index: i,
                size:  frame_size,
                data:  Bytes::from(vec![0u8; frame_size]),
            });
        }
    }
    done.store(true, Ordering::Release);

    (buffer, done)
}

// ── Benchmark 1: distintas cantidades de frames ───────────────────────────
//
// Mide cuánto tarda consume() con 10, 100 y 1000 frames.
// BenchmarkId permite comparar distintos tamaños en el mismo gráfico.
fn bench_consume_por_cantidad(c: &mut Criterion) {
    // Runtime (entorno de ejecución) de Tokio para funciones async en benchmarks
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("consume_por_cantidad");

    for n_frames in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(n_frames), // etiqueta del benchmark
            &n_frames,
            |b, &n| {
                // to_async: ejecutar el benchmark en el runtime de Tokio
                b.to_async(&rt).iter(|| async {
                    let (buffer, done) = preparar_buffer(n, 65536);
                    consume(buffer, done).await.unwrap()
                });
            },
        );
    }

    group.finish();
}

// ── Benchmark 2: distintos tamaños de frame ───────────────────────────────
//
// Mide el impacto del tamaño del frame en el rendimiento del consumidor.
// Fijamos 100 frames y variamos el tamaño: 4 KB, 32 KB, 64 KB.
fn bench_consume_por_tamanio(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("consume_por_tamanio_frame");

    for frame_size in [4096, 32768, 65536] {
        group.bench_with_input(
            BenchmarkId::from_parameter(frame_size),
            &frame_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let (buffer, done) = preparar_buffer(100, size);
                    consume(buffer, done).await.unwrap()
                });
            },
        );
    }

    group.finish();
}

// criterion_group! registra los grupos de benchmarks a correr.
// criterion_main! genera el punto de entrada del binario de benchmarks.
criterion_group!(benches, bench_consume_por_cantidad, bench_consume_por_tamanio);
criterion_main!(benches);
