use std::env;
use std::error::Error;

/**
 *** Objective of this file is to create a new index with some configs before passing them to plugin manager
 *** Also provide some endpoints to get the index's detail
 **/
// Generic dependencies
use diesel::{Connection, PgConnection};
use lazy_static::lazy_static;
use log::{debug, info, warn};
use serde_yaml::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_compat_02::FutureExt;

// Massbit dependencies
use crate::adapter::adapter_init;
use crate::config::{generate_random_hash, get_index_name};
use crate::config_builder::{IndexConfigIpfsBuilder, IndexConfigLocalBuilder};
use crate::ddl_gen::run_ddl_gen;
use crate::hasura::track_hasura_with_ddl_gen_plugin;
use crate::ipfs::{download_ipfs_file_by_hash, read_config_file};
use crate::type_index::{IndexStore, Indexer};
use crate::type_request::DeployParams;
use adapter::core::AdapterManager;

// Graph dependencies
use graph::data::subgraph::UnresolvedSubgraphManifest;
use graph::data::subgraph::SPEC_VERSION_0_0_4;
use graph::data::subgraph::{SubgraphAssignmentProviderError, SubgraphManifest};
use graph::ipfs_client::IpfsClient;
use graph::log::logger;
use graph_chain_ethereum::{Chain, DataSource};
use graph_core::LinkResolver;

lazy_static! {
    static ref CHAIN_READER_URL: String =
        env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
    static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
}

pub async fn start_new_index(params: DeployParams) -> Result<(), Box<dyn Error>> {
    let index_config = IndexConfigIpfsBuilder::default()
        .config(&params.config)
        .await
        .mapping(&params.mapping)
        .await
        .schema(&params.schema)
        .await
        .abi(params.abi)
        .await
        .subgraph(&params.subgraph)
        .await
        .build();

    // TODO: Maybe break this into two different struct (So and Wasm) so we don't have to use Option
    let manifest: Option<SubgraphManifest<Chain>> = match &params.subgraph {
        Some(v) => Some(get_manifest(v).await.unwrap()),
        None => {
            println!(".SO mapping doesn't have parsed data source");
            //vec![]
            None
        }
    };

    // Create tables for the new index and track them in hasura
    //run_ddl_gen(&index_config).await;

    // Create a new indexer so we can keep track of it's status
    IndexStore::insert_new_indexer(&index_config);

    // Start the adapter for the index
    adapter_init(&index_config, &manifest).await?;

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
            // adapter_init(&index_config).await;
            // TODO: Enable new index Config so we can have the start index on restart
        });
    }
    Ok(())
}

// Return indexer list
pub async fn list_handler_helper() -> Result<Vec<Indexer>, Box<dyn Error>> {
    let indexers = IndexStore::get_indexer_list();
    Ok(indexers)
}

/********* HELPER FUNCTION ************/
// TODO: Move to a different file
async fn get_manifest(
    file_hash: &String,
) -> Result<SubgraphManifest<Chain>, SubgraphAssignmentProviderError> {
    let logger = logger(true);
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;

    let file_bytes = ipfs_clients[0]
        .cat_all(file_hash.to_string(), Duration::from_secs(10))
        .compat()
        .await
        .unwrap()
        .to_vec();

    // Get raw manifest
    let file = String::from_utf8(file_bytes).unwrap();

    println!("File: {}", file);

    let raw: serde_yaml::Value = serde_yaml::from_str(&file).unwrap();

    let mut raw_manifest = match raw {
        serde_yaml::Value::Mapping(m) => m,
        _ => panic!("Wrong type raw_manifest"),
    };

    // Inject the IPFS hash as the ID of the subgraph into the definition.
    let id = "deployment_hash";
    raw_manifest.insert(
        serde_yaml::Value::from("id"),
        serde_yaml::Value::from(id.to_string()),
    );

    // Parse the YAML data into an UnresolvedSubgraphManifest
    let value: Value = raw_manifest.into();
    let unresolved: UnresolvedSubgraphManifest<Chain> = serde_yaml::from_value(value).unwrap();
    let resolver = Arc::new(LinkResolver::from(ipfs_clients));

    debug!("Features {:?}", unresolved.features);
    let manifest = unresolved
        .resolve(&*resolver, &logger, SPEC_VERSION_0_0_4.clone())
        .compat()
        .await
        .map_err(SubgraphAssignmentProviderError::ResolveError)?;

    //println!("data_sources: {:#?}", &manifest.data_sources);
    Ok(manifest)
}

/********* HELPER FUNCTION ************/
// TODO: Move to a different file
pub async fn create_ipfs_clients(ipfs_addresses: &Vec<String>) -> Vec<IpfsClient> {
    // Parse the IPFS URL from the `--ipfs` command line argument
    let ipfs_addresses: Vec<_> = ipfs_addresses
        .iter()
        .map(|uri| {
            if uri.starts_with("http://") || uri.starts_with("https://") {
                String::from(uri)
            } else {
                format!("http://{}", uri)
            }
        })
        .collect();

    ipfs_addresses
        .into_iter()
        .map(|ipfs_address| {
            log::info!("Connecting to IPFS node");
            let ipfs_client = match IpfsClient::new(&ipfs_address) {
                Ok(ipfs_client) => ipfs_client,
                Err(e) => {
                    log::error!("Failed to create IPFS client {}", e);
                    panic!("Could not connect to IPFS");
                }
            };
            ipfs_client
        })
        .collect()
}
