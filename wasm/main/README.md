## Prerequisites

```shell
rustup target add wasm32-wasi
cd massbitprotocol/wasm/main
rustc src/main.rs --target wasm32-wasi
wasmtime main.wasm
```

Or use .wasm file from assemblyscript
```shell
wasmtime ../assembly-example/index.wasm
```
