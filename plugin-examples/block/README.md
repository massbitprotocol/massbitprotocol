### Plugin setup 
```shell
cargo install diesel_cli
export DATABASE_URL="postgres://postgres:postgres@localhost"
diesel migration run
cargo build --release
```

### Plugin manager setup
```rust
use plugin::PluginManager;
use std::{path::PathBuf};
use massbit_chain_substrate::data_type::SubstrateBlock;
...

let library_path = PathBuf::from("path to *.so file".to_string());
let mut plugins = PluginManager::new();
unsafe {
    plugins
        .load(&library_path)
        .expect("plugin loading failed");
}
plugins.handle_block(&block);
```