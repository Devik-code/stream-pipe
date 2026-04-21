// ═══════════════════════════════════════════════════════════════
// ARCHIVO: frame.rs — define qué es un Frame (cuadro de video)
//
// Peras y manzanas:
//   Un video es como un libro de historietas (flipbook):
//   miles de imágenes dibujadas una por página. Si las pasás rápido,
//   parece movimiento. Cada página es un frame (cuadro).
//
//   En este programa NO tenemos imágenes reales — simulamos frames
//   cortando el archivo de video en pedazos de 64KB (kilobytes —
//   unidad de medida de datos, 1KB = 1024 bytes — unidades mínimas de datos).
//   Es como cortar el libro en grupos de 100 páginas sin mirar el dibujo.
// ═══════════════════════════════════════════════════════════════

// Bytes es un tipo (clase de dato) de la librería (biblioteca de código) `bytes`.
// Internamente funciona como un Arc<[u8]>
use bytes::Bytes;

// Frame (cuadro): la unidad de trabajo que viaja por el pipeline (tubería de datos).
//
// Peras y manzanas:
//   Cada Frame es como una caja en una cinta transportadora de fábrica.
//   La caja tiene una etiqueta con su número (#1, #2, #3...),
//   dice cuánto pesa (size — tamaño en bytes) y adentro tiene la mercadería
//   (data — los bytes del video).
//
// pub struct = estructura de datos pública (visible desde otros módulos —
//              archivos de código del proyecto).
//              En C++ sería: struct Frame { ... };
//              En Python sería: class Frame: ...
pub struct Frame {
    // index (índice): número de orden del frame, empezando desde 0.
    // Sirve para saber si los frames llegan en orden correcto.
    // Peras y manzanas: es como el número de página del libro.
    // usize = número entero positivo (sin signo — sin negativo)
    pub index: usize,

    // size (tamaño): cantidad real de bytes en este frame (cuadro).
    // Normalmente es igual a FRAME_SIZE (65536 bytes — 64KB),
    // excepto el último frame del video que puede ser más chico
    // si el archivo no es múltiplo exacto de 64KB.
    // Peras y manzanas: el peso real de la caja — casi siempre 64KB,
    // pero la última caja puede venir a medio llenar.
    pub size: usize,

    // data (datos): los bytes reales del video dentro de este frame.
    //
    // Por qué Bytes y no Vec<u8> (vector — lista dinámica — de bytes)?
    //   Vec<u8> es como una caja de madera pesada: al compartirla entre hilos
    //   (threads — unidades de ejecución paralela) necesitás copiarla entera.
    //   Bytes es como una caja de cartón con una etiqueta de "dueño":
    //   podés pasarla entre hilos (threads) sin copiar el contenido —
    //   solo se transfiere la etiqueta (el puntero — dirección de memoria).
    //
    // Bytes es Send (enviable entre hilos — el compilador garantiza que es seguro
    //               moverlo de un hilo a otro) y Sync (sincronizable — múltiples
    //               hilos pueden leerlo al mismo tiempo sin problemas).
    pub data: Bytes,
}
