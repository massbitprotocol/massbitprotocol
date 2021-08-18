use std::env;
use std::error::Error;

/**
 *** Objective of this file is to create a new index with some configs before passing them to plugin manager
 *** Also provide some endpoints to get the index's detail
 **/
// Generic dependencies
use diesel::{Connection, PgConnection};
use lazy_static::lazy_static;

use adapter::core::AdapterManager;

// Massbit dependencies
use crate::adapter::adapter_init;
use crate::config::{generate_random_hash, get_index_name};
use crate::config_builder::{IndexConfigIpfsBuilder, IndexConfigLocalBuilder};
use crate::ddl_gen::run_ddl_gen;
use crate::hasura::track_hasura_with_ddl_gen_plugin;
use crate::ipfs::{get_ipfs_file_by_hash, read_config_file};
use crate::types::{DeployParams, IndexStore, Indexer};

lazy_static! {
    static ref CHAIN_READER_URL: String =
        env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
    static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
}

pub async fn start_new_index(params: DeployParams) -> Result<(), Box<dyn Error>> {
    // Get user index mapping logic, query for migration and index's configurations
    // TODO: Parse the config so we know what type of mapping are we dealing with
    // TODO: Add a new struct for mapping value
    let index_config = IndexConfigIpfsBuilder::default()
        .config(&params.config)
        .await
        .mapping(&params.mapping)
        .await
        .schema(&params.schema)
        .await
        .build();

    // Create tables for the new index and track them in hasura
    run_ddl_gen(&index_config).await;

    // Create a new indexer so we can keep track of it's status
    IndexStore::insert_new_indexer(&index_config);

    // Start the adapter for the index
    adapter_init(&index_config).await;

    Ok(())
}

pub async fn restart_all_existing_index_helper() -> Result<(), Box<dyn Error>> {
    let indexers = IndexStore::get_indexer_list();

    if indexers.len() == 0 {
        log::info!("No index found");
        return Ok(());
    }

    for indexer in indexers {
        tokio::spawn(async move {
            let index_config = IndexConfigLocalBuilder::default()
                .config(&indexer.hash)
                .await
                .mapping(&indexer.hash)
                .await
                .schema(&indexer.hash)
                .await
                .build();
            adapter_init(&index_config).await;
        });
    }
    Ok(())
}

// Return indexer list
pub async fn list_handler_helper() -> Result<Vec<Indexer>, Box<dyn Error>> {
    let indexers = IndexStore::get_indexer_list();
    Ok(indexers)
}
