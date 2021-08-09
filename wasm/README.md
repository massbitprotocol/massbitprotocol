## Assembly script Compiler

Assembly script compiler isn't published in NPM, so we need to pull the compiler from the github repo
```shell
git clone https://github.com/AssemblyScript/assemblyscript
cd assemblyscript
```

Build
```shell
assemblyscript/bin/asc assembly-example/assembly/index.ts --textFile > assembly-example/assembly/index.wat
assemblyscript/bin/asc assembly-example/assembly/index.ts --binaryFile > assembly-example/assembly/index.wasm
```


## Rust WASM Time

```shell
rustup target add wasm32-wasi
rustc main/src/main.rs --target wasm32-wasi
```

Use .wasm file from assemblyscript
```shell
wasmtime assembly-example/assembly/index.wasm
```

Or use .wasm file from Rust
```shell
wasmtime main/main.wasm
```