// build.rs — script de compilación (build script)
//
// Peras y manzanas:
//   Este archivo corre ANTES de compilar el proyecto.
//   Es como el electricista que conecta los cables antes de que lleguen los
//   muebles — prepara lo que el linker (enlazador — une las piezas compiladas
//   en un ejecutable) necesita saber.
//
// Problema que resuelve:
//   Rust 1.87+ usa GetHostNameW (función de red — obtener el nombre del host
//   en Windows) que vive en ws2_32 (Windows Sockets 2 — biblioteca de red
//   de Windows). Al compilar cruzado (cross-compile — desde Linux para Windows)
//   con la toolchain GNU (GNU's Not Unix — cadena de herramientas libre),
//   el linker no la enlaza automáticamente.
//
// Solución:
//   cargo:rustc-link-lib=ws2_32 le dice a Cargo (el gestor de paquetes de Rust)
//   "al linkear para Windows, incluí la biblioteca ws2_32".
//   Cargo la ubica en el lugar correcto del comando de linkeo — DESPUÉS de libstd.

fn main() {
    // CARGO_CFG_TARGET_OS = variable de entorno (environment variable — valor
    // del sistema que Cargo define automáticamente durante la compilación)
    // con el sistema operativo destino: "windows", "linux", "macos", etc.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // cargo:rustc-link-lib=ws2_32 = "enlazar con ws2_32 al compilar para Windows"
        // Sin este flag, GetHostNameW queda como "undefined reference" (referencia sin resolver)
        // y el ejecutable (binario — archivo .exe) no se puede generar.
        println!("cargo:rustc-link-lib=ws2_32");
    }
}
