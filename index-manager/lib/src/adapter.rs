use crate::ipfs::read_config_file;
use crate::type_index::IndexConfig;
use adapter::core::AdapterManager;
use std::error::Error;







use graph::prelude::SubgraphManifest;
use graph_chain_ethereum::{Chain};




pub async fn adapter_init(
    index_config: &IndexConfig,
    manifest: &Option<SubgraphManifest<Chain>>,
    got_block: Option<i64>
) -> Result<(), Box<dyn Error>> {
    log::info!("Load library from {:?}", &index_config.mapping);
    let config_value = read_config_file(&index_config.config);
    let mut adapter = AdapterManager::new();
    //assert_eq!(manifest.data_sources.len(), 1);

    println!("{:?}", index_config);
    adapter
        .init(
            &index_config.identifier.name_with_hash,
            &config_value,
            &index_config.mapping,
            &index_config.schema,
            manifest,
            got_block
        )
        .await?;
    Ok(())
}
