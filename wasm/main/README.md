## Prerequisites

```shell
rustup target add wasm32-wasi
cd massbitprotocol/wasm/main
rustc src/main.rs --target wasm32-wasi
wasmtime main.wasm
```