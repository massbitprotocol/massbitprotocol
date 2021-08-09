use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::from_file(&engine, "assembly-example/hello-world/build/optimized.wasm")?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let run = instance.get_typed_func::<(), (i32), _>(&mut store, "run").unwrap();
    run.call(&mut store, ())?;
    Ok(())
}