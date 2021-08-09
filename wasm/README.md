## Create new AssemblyScript example
```shell
cd assembly-example
npm install -g assemblyscript
asinit hello-world
cd hello-world
npm install --save as-wasi
npm run asbuild
```

## Rust WASM Time
Prerequisites
```shell
rustup target add wasm32-wasi
```

Run .wasm file from assemblyscript-example with CLI
```shell
wasmtime assembly-example/hello-world/build/optimized.wasm
```
Run .wasm file from assemblyscript-example with Rust code
```shell
cargo run --bin wasm-main
```

## Reference
https://github.com/bytecodealliance/wasmtime/tree/main/examples