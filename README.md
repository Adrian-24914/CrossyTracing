# Crossy Tracing

Video demostración:

https://github.com/user-attachments/assets/170e44e3-811f-49c3-bbb9-f8fb2d74b8e0

## Ejecutar en Windows

Desde la raíz del proyecto:

```powershell
cargo run --release
```

## Ejecutar en el navegador

### Preparación inicial

Instala el target de WebAssembly y la versión de `wasm-bindgen-cli` utilizada por el proyecto:

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
```

Estos comandos solamente son necesarios la primera vez.

### Compilar y generar el paquete web

Ejecuta desde la raíz del proyecto:

```powershell
cargo build --release --target wasm32-unknown-unknown --lib
wasm-bindgen --target web --out-dir web/pkg --no-typescript target/wasm32-unknown-unknown/release/crossy_tracing.wasm
```

El contenido generado queda en `web/pkg/` y no se guarda en Git.

### Iniciar el servidor local

```powershell
python -m http.server 8000 --directory web
```

Después abre [http://127.0.0.1:8000](http://127.0.0.1:8000) en el navegador. Para detener el servidor utiliza `Ctrl+C`.

El archivo `web/index.html` no debe abrirse directamente desde el explorador de archivos: los módulos JavaScript y WebAssembly necesitan servirse mediante HTTP.

