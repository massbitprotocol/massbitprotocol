use crate::ipfs::read_config_file;
use crate::type_index::IndexConfig;
use adapter::core::AdapterManager;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use log::{debug, info, warn, Level};

use crate::config::get_mapping_language;
use graph::prelude::SubgraphManifest;
use graph_chain_ethereum::{Chain, DataSource};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

pub async fn adapter_init(
    index_config: &IndexConfig,
    manifest: &Option<SubgraphManifest<Chain>>,
) -> Result<(), Box<dyn Error>> {
    log::info!("Load library from {:?}", &index_config.mapping);
    let config_value = read_config_file(&index_config.config);
    let mut adapter = AdapterManager::new();
    //assert_eq!(manifest.data_sources.len(), 1);

    println!("Index config {:?}", index_config);
    adapter
        .init(
            &index_config.identifier.name_with_hash,
            &config_value,
            &index_config.mapping,
            &index_config.schema,
            manifest,
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
