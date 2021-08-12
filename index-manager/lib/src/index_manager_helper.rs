/**
*** Objective of this file is to create a new index with some configs before passing them to plugin manager
*** Also provide some endpoints to get the index's detail
**/
// Generic dependencies
use diesel::{Connection, PgConnection};
use lazy_static::lazy_static;
use std::env;
use std::error::Error;

// Massbit dependencies
use crate::config::{generate_random_hash, get_index_name};
use crate::config_builder::IndexConfigIpfsBuilder;
use crate::ddl_gen::run_ddl_gen;
use crate::hasura::track_hasura_with_ddl_gen_plugin;
use crate::ipfs::{get_ipfs_file_by_hash, read_config_file};
use crate::types::{DeployParams, IndexStore, Indexer};
use adapter::core::AdapterManager;

lazy_static! {
    static ref CHAIN_READER_URL: String =
        env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
    static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
}

pub async fn loop_blocks(params: DeployParams) -> Result<(), Box<dyn Error>> {
    // Get user index mapping logic, query for migration and index's configurations
    let index_config = IndexConfigIpfsBuilder::default()
        .config(&params.config)
        .await
        .mapping(&params.mapping)
        .await
        .schema(&params.schema)
        .await
        .build();

    // Parse config file
    let config_value = read_config_file(&index_config.config);

    // Create tables for the new index and track them in hasura
    run_ddl_gen(&index_config).await;

    // Create a new indexer so we can keep track of it's status
    IndexStore::insert_new_indexer(&index_config.identifier.name_with_hash, &config_value);

    // Chain Reader Client Configuration to subscribe and get latest block from Chain Reader Server
    log::info!("Load library from {:?}", &index_config.mapping);
    let mut adapter = AdapterManager::new();
    adapter
        .init(
            &index_config.identifier.name_with_hash,
            &config_value,
            &index_config.mapping,
        )
        .await;
    Ok(())
}

// Return indexer list
pub async fn list_handler_helper() -> Result<Vec<Indexer>, Box<dyn Error>> {
    let indexers = IndexStore::get_indexer_list();
    Ok(indexers)
}
