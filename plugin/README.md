# Plugin manager

## Usage
```rust
use plugin_manager::PluginManager;
use std::{path::PathBuf};
use types::{SubstrateBlock};
...

let library_path = PathBuf::from("path to libraty".to_string());
let mut plugins = PluginManager::new();
unsafe {
    plugins
        .load(&library_path)
        .expect("Function loading failed");
}

let block = SubstrateBlock { idx: 1 };
plugins.handle_block(&block);
```