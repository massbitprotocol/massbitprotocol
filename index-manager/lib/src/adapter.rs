use crate::ipfs::read_config_file;
use crate::type_index::IndexConfig;
use adapter::core::AdapterManager;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

const QUICKSWAP_PATH: &str = r#"/home/viettai/Massbit/QuickSwap-subgraph"#;
const WASM_FILE: &str = r#"build/Factory/Factory.wasm"#;
const MANIFEST: &str = r#"subgraph.yaml"#;
const SCHEMA: &str = r#"schema.graphql"#;
use log::{debug, info, warn, Level};

use crate::config::get_mapping_language;
use graph_chain_ethereum::DataSource;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

pub async fn adapter_init(
    index_config: &IndexConfig,
    data_sources: &Vec<DataSource>,
) -> Result<(), Box<dyn Error>> {
    // Chain Reader Client Configuration to subscribe and get latest block from Chain Reader Server
    //let config_value = read_config_file(&index_config.config);
    let config_path = PathBuf::from(format!("{}/{}", QUICKSWAP_PATH, MANIFEST).as_str());
    let config_value = read_config_file(&config_path);
    log::info!("Load library from {:?}", &index_config.mapping);
    let runtime_path = PathBuf::from(format!("{}/{}", QUICKSWAP_PATH, WASM_FILE).as_str());
    let schema_path = PathBuf::from(format!("{}/{}", QUICKSWAP_PATH, SCHEMA).as_str());
    let mut adapter = AdapterManager::new();
    assert_eq!(data_sources.len(), 1);
    adapter
        .init(
            &index_config.identifier.name_with_hash,
            &config_value,
            &index_config.mapping,
            &index_config.schema,
        )
        .await;
    /*
    if get_mapping_language(&config_value).to_string().contains("wasm") {
        log::info!("Handling .wasm file");
        // TODO: we have the datasource, now the handler can get the ethereum event

    } else {
        log::info!("Handling .so file");
        let mut adapter = AdapterManager::new();
        adapter
            .init(
                &index_config.identifier.name_with_hash,
                &config_value,
                &index_config.mapping,
            )
            .await;
    }
    */
    Ok(())
}
