# stream-pipe

Pipeline de streaming de video: descarga un video por HTTP, lo procesa en chunks (pedazos) simulando frames (cuadros) y los consume en paralelo con workers (trabajadores) configurables.

---

## Requerimientos

- **Rust** — compilador y cargo
- **cross** — compilacion cruzada (compilar para otro SO desde Linux)
- **Docker** — requerimiento de cross, provee el entorno con el toolchain correcto
- **just** — task runner para ejecutar comandos del Justfile en `tools/`
- **cargo-deb** — genera paquetes `.deb` para Debian/Ubuntu
- **cargo-deny** — verifica licencias y vulnerabilidades

---

## Preparacion

**Instalar Rust:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Instalar herramientas:**
```bash
cargo install cross
cargo install just
cargo install cargo-deb
cargo install cargo-deny
```

**Docker** debe estar instalado y corriendo antes de usar `cross`.
- Ubuntu: https://docs.docker.com/engine/install/ubuntu/
- Verificar que corre:
```bash
docker info
```

---

## Compilacion

### Linux x86_64 (nativo)
```bash
cargo build --release
# Ejecutable: target/release/stream-pipe
```

### Compilacion cruzada (cross-compilation)

`cross` usa Docker para tener el toolchain (compilador + linker) correcto para cada plataforma. Requiere Docker corriendo.

**Windows 64-bit**
```bash
just -f tools/Justfile build-windows
# o directamente:
cross build --release --target x86_64-pc-windows-gnu
# Ejecutable: target/x86_64-pc-windows-gnu/release/stream-pipe.exe
```

> **Nota:** se requiere la imagen `:main` de cross (no `:0.2.4`) porque Rust 1.87+
> usa `GetHostNameW` que necesita `ws2_32`. `Cross.toml` ya esta configurado correctamente.

**Linux ARM 64-bit — Raspberry Pi 4/5, servidores ARM**
```bash
just -f tools/Justfile build-linux-arm64
# o directamente:
cross build --release --target aarch64-unknown-linux-gnu
# Ejecutable: target/aarch64-unknown-linux-gnu/release/stream-pipe
```

**Linux ARM 32-bit — Raspberry Pi 2/3**
```bash
just -f tools/Justfile build-linux-arm32
# o directamente:
cross build --release --target arm-unknown-linux-gnueabihf
# Ejecutable: target/arm-unknown-linux-gnueabihf/release/stream-pipe
```

**Linux 64-bit estatico musl — cualquier Linux, contenedores Docker**
```bash
just -f tools/Justfile build-linux-musl
# o directamente:
cross build --release --target x86_64-unknown-linux-musl
# Ejecutable: target/x86_64-unknown-linux-musl/release/stream-pipe
```

**Todas las plataformas de una vez**
```bash
just -f tools/Justfile build-all
```

---

## Justfile

Los comandos mas comunes estan en `tools/Justfile`. Requiere `just` instalado.

```bash
just -f tools/Justfile --list          # ver todos los comandos disponibles
just -f tools/Justfile ci              # fmt + check + clippy + test + deny
just -f tools/Justfile run             # compilar y correr el programa
just -f tools/Justfile build-windows   # compilar cruzado para Windows
```

---

## Generar paquete .deb

Requiere tener el binario compilado primero para cada target. Solo aplica a plataformas Linux (no Windows).

```bash
# Linux x86_64 (nativo)
cargo build --release
cargo deb --no-build -v
# Paquete: target/debian/stream-pipe_<version>_amd64.deb

# Linux ARM 64-bit (Raspberry Pi 4/5, servidores ARM)
cross build --release --target aarch64-unknown-linux-gnu
cargo deb --no-build --target aarch64-unknown-linux-gnu -v --no-strip
# Paquete: target/aarch64-unknown-linux-gnu/debian/stream-pipe_<version>_arm64.deb

# Linux ARM 32-bit (Raspberry Pi 2/3)
cross build --release --target arm-unknown-linux-gnueabihf
cargo deb --no-build --target arm-unknown-linux-gnueabihf -v --no-strip
# Paquete: target/arm-unknown-linux-gnueabihf/debian/stream-pipe_<version>_armhf.deb

# Linux 64-bit estatico musl
cross build --release --target x86_64-unknown-linux-musl
cargo deb --no-build --target x86_64-unknown-linux-musl -v
# Paquete: target/x86_64-unknown-linux-musl/debian/stream-pipe_<version>_amd64.deb
```

---

## Configuracion

El programa lee `config.toml` en la raiz del proyecto (desarrollo local)
o `/var/lib/stream-pipe/config.toml` (produccion — servidor instalado).

```toml
[video]
url        = "http://..."   # URL del video a descargar
frame_size = 65536          # tamanio de cada chunk en bytes (64 KB)

[pipeline]
workers = 1                 # cantidad de consumidores en paralelo

[logging]
level   = "info"            # error | warn | info | debug | trace
log_dir = "logs"            # carpeta donde se guardan los archivos de log
```

---

## Logs

Los logs se guardan en la carpeta definida en `config.toml` (`log_dir`).

| Entorno     | Ruta                      |
|-------------|---------------------------|
| Desarrollo  | `logs/`                   |
| Produccion  | `/var/log/stream-pipe/`   |

Formato: `stream-pipe.YYYY-MM-DD.log` — rotacion diaria, ultimos 14 dias.
