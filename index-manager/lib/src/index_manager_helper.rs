use anyhow::Context;
/**
 *** Objective of this file is to create a new index with some configs before passing them to plugin manager
 *** Also provide some endpoints to get the index's detail
 **/
// Generic dependencies
use diesel::{Connection, PgConnection};
use std::env;
use std::error::Error;
use std::hash::Hash;
use std::ops::Deref;

use lazy_static::lazy_static;
use log::{debug, error, info};
use serde_yaml::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio_compat_02::FutureExt;

// Massbit dependencies
use crate::adapter::adapter_init;

use crate::config_builder::{IndexConfigIpfsBuilder, IndexConfigLocalBuilder};

use crate::ipfs::read_config_file;
use crate::type_index::{IndexStore, Indexer};
use crate::type_request::DeployParams;

// Graph dependencies
use graph::data::subgraph::UnresolvedSubgraphManifest;
use graph::data::subgraph::SPEC_VERSION_0_0_4;
use graph::data::subgraph::{SubgraphAssignmentProviderError, SubgraphManifest};
//use graph::ipfs_client::IpfsClient;
//use graph_core::LinkResolver;
use chain_solana::chain::Chain;
use index_store::indexer::IndexerStore;
use massbit::cheap_clone::CheapClone;
use massbit::components::store::{DeploymentId, DeploymentLocator};
use massbit::data::indexer::MAX_SPEC_VERSION;
use massbit::data::indexer::{IndexerManifestResolveError, IndexerRegistrarError};
use massbit::ipfs_client::IpfsClient;
use massbit::ipfs_link_resolver::LinkResolver;
use massbit::log::logger;
use massbit::prelude::{DeploymentHash, IndexerManifest, LinkResolver as _, LoggerFactory};
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
        //.abi(params.abi)
        //.await
        .subgraph(&params.subgraph)
        .await
        .build();
    // Set up logger
    let logger = logger(false);
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    let ipfs_clients: Vec<IpfsClient> = create_ipfs_clients(&ipfs_addresses).await;

    // Convert the clients into a link resolver. Since we want to get past
    // possible temporary DNS failures, make the resolver retry
    let link_resolver = Arc::new(LinkResolver::from(ipfs_clients));
    // Create a component and indexer logger factory
    let logger_factory = LoggerFactory::new(logger.clone());
    let deployment_hash = DeploymentHash::new(index_config.identifier.hash.clone())?;
    let logger = logger_factory.indexer_logger(&DeploymentLocator::new(
        DeploymentId(0),
        deployment_hash.clone(),
    ));
    info!("Ipfs {:?}", &deployment_hash.to_ipfs_link());
    // let raw: serde_yaml::Mapping = {
    //     let file_bytes = link_resolver
    //         .cat(&logger, &deployment_hash.to_ipfs_link())
    //         .await
    //         .map_err(|e| {
    //             error!("{:?}", &e);
    //             IndexerRegistrarError::ResolveError(IndexerManifestResolveError::ResolveError(e))
    //         })?;
    //
    //     serde_yaml::from_slice(&file_bytes)
    //         .map_err(|e| IndexerRegistrarError::ResolveError(e.into()))?
    // };
    // TODO: Maybe break this into two different struct (So and Wasm) so we don't have to use Option
    // let mut manifest = IndexerManifest::<Chain>::resolve_from_raw(
    //     &logger,
    //     deployment_hash.cheap_clone(),
    //     raw,
    //     // Allow for infinite retries for indexer definition files.
    //     &link_resolver.as_ref().clone().with_retries(),
    //     MAX_SPEC_VERSION.clone(),
    // )
    // .await
    // .context("Failed to resolve indexer from IPFS")?;
    let manifest: Option<IndexerManifest<Chain>> = match &params.subgraph {
        Some(v) => Some(
            get_indexer_manifest(DeploymentHash::new(v)?, link_resolver)
                .await
                .unwrap(),
        ),
        None => {
            println!(".SO mapping doesn't have parsed data source");
            //vec![]
            None
        }
    };
    // Create tables for the new index and track them in hasura
    //run_ddl_gen(&index_config).await;

    // Create a new indexer so we can keep track of it's status
    //IndexStore::insert_new_indexer(&index_config);
    let config_value = read_config_file(&index_config.config);
    let network = config_value["dataSources"][0]["kind"].as_str().unwrap();
    let name = config_value["dataSources"][0]["name"].as_str().unwrap();
    IndexerStore::create_indexer(
        index_config.identifier.hash.clone(),
        String::from(name),
        String::from(network),
        &params.subgraph,
    );
    // Start the adapter for the index
    adapter_init(&index_config, &manifest).await?;

    Ok(())
}

pub async fn restart_all_existing_index_helper() -> Result<(), Box<dyn Error>> {
    let database_url = crate::DATABASE_CONNECTION_STRING.as_str();
    let _conn = PgConnection::establish(database_url)
        .expect(&format!("Error connecting to {}", database_url));
    //let indexers = IndexStore::get_indexer_list();
    let indexers = IndexerStore::get_active_indexers();
    if indexers.len() == 0 {
        log::info!("No index found");
        return Ok(());
    }

    for indexer in indexers {
        tokio::spawn(async move {
            let index_config = IndexConfigLocalBuilder::default()
                .hash(&indexer.hash)
                .config(&indexer.hash)
                .await
                .mapping(&indexer.hash)
                .await
                .schema(&indexer.hash)
                .await
                .build();
            // if indexer.manifest.as_str() != "" {
            //     let manifest: Option<SubgraphManifest<Chain>> =
            //         Some(get_manifest(&indexer.manifest).await.unwrap());
            //     let start_block = if indexer.got_block > 0 {
            //         Some(indexer.got_block)
            //     } else {
            //         None
            //     };
            //     adapter_init(&index_config, &manifest, start_block).await;
            // }
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
async fn get_indexer_manifest(
    deployment_hash: DeploymentHash,
    link_resolver: Arc<LinkResolver>, //file_hash: &String,
) -> Result<IndexerManifest<Chain>, SubgraphAssignmentProviderError> {
    let logger = logger(true);
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;

    let file_bytes = ipfs_clients[0]
        .cat_all(&deployment_hash.to_string(), Some(Duration::from_secs(10)))
        .compat()
        .await
        .unwrap()
        .to_vec();

    // let file = String::from_utf8(file_bytes).unwrap();
    //
    // println!("File: {}", file);

    let raw: serde_yaml::Value = serde_yaml::from_slice(&file_bytes).unwrap();

    let mut raw_manifest = match raw {
        serde_yaml::Value::Mapping(m) => m,
        _ => panic!("Wrong type raw_manifest"),
    };
    // Get raw manifest
    let mut manifest = IndexerManifest::<Chain>::resolve_from_raw(
        &logger,
        deployment_hash.cheap_clone(),
        raw_manifest,
        // Allow for infinite retries for indexer definition files.
        &link_resolver.as_ref().clone().with_retries(),
        MAX_SPEC_VERSION.clone(),
    )
    .await
    .context("Failed to resolve indexer from IPFS")?;

    // Parse the YAML data into an UnresolvedSubgraphManifest
    // let value: Value = raw_manifest.into();
    // let unresolved: UnresolvedSubgraphManifest<Chain> = serde_yaml::from_value(value).unwrap();
    // let resolver = Arc::new(LinkResolver::from(ipfs_clients));
    //
    // debug!("Features {:?}", unresolved.features);
    // let manifest = unresolved
    //     .resolve(&*resolver, &logger, SPEC_VERSION_0_0_4.clone())
    //     .compat()
    //     .await
    //     .map_err(SubgraphAssignmentProviderError::ResolveError)?;

    //println!("data_sources: {:#?}", &manifest.data_sources);
    Ok(manifest)
}

/*
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
*/
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
            log::info!("Create IPFS client with addresses: {:?}", &ipfs_address);
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
