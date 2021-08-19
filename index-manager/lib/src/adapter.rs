use crate::ipfs::read_config_file;
use crate::types::IndexConfig;
use adapter::core::AdapterManager;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

const QUICKSWAP_PATH: &str = r#"/home/viettai/Massbit/QuickSwap-subgraph"#;
const WASM_FILE: &str = r#"build/Factory/Factory.wasm"#;
const MANIFEST: &str = r#"subgraph.yaml"#;

use log::{debug, info, warn, Level};

use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

pub async fn adapter_init(index_config: &IndexConfig) -> Result<(), Box<dyn Error>> {
    // Chain Reader Client Configuration to subscribe and get latest block from Chain Reader Server
    //let config_value = read_config_file(&index_config.config);
    let config_path = PathBuf::from(format!("{}/{}", QUICKSWAP_PATH, MANIFEST).as_str());
    let config_value = read_config_file(&config_path);
    log::info!("Load library from {:?}", &index_config.mapping);
    let runtime_path = PathBuf::from(format!("{}/{}", QUICKSWAP_PATH, WASM_FILE).as_str());
    let mut adapter = AdapterManager::new();
    adapter
        .init(
            &index_config.identifier.name_with_hash,
            &config_value,
            &runtime_path,
            //&index_config.mapping,
        )
        .await;
    Ok(())
}
