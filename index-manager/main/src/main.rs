use lazy_static::lazy_static;
/**
 *** Objective of this file is to  
 *** - start a JSON HTTP SERVER to receive the index requests and expose some API about indexers
 *** - support restarting indexers
 **/
// Generic dependencies
use std::env;

// Massbit dependencies
use index_manager_lib::index_manager::IndexManager;
use logger::core::init_logger;
use graph_chain_ethereum;
use std::sync::Arc;
use graph_core::{
    LinkResolver, MetricsRegistry, SubgraphAssignmentProvider as IpfsSubgraphAssignmentProvider,
    SubgraphInstanceManager, SubgraphRegistrar as IpfsSubgraphRegistrar,
};
use log::{info};
use ethereum::{EthereumNetworks, NodeCapabilities, ProviderEthRpcMetrics};
use futures::future::join_all;
use git_testament::{git_testament, render_testament};
use graph::{ipfs_client::IpfsClient, prometheus::Registry};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use std::{collections::HashMap};
use structopt::StructOpt;

use graph::blockchain::block_ingestor::BlockIngestor;
use graph::blockchain::Blockchain as _;
use graph::components::store::BlockStore;
use graph::data::graphql::effort::LoadManager;
use graph::log::logger;
use graph::prelude::{IndexNodeServer as _, JsonRpcServer as _};
use graph::util::security::SafeDisplay;
use graph_chain_ethereum::{self as ethereum, network_indexer, EthereumAdapterTrait, Transport};

mod config;
mod opt;
mod store_builder;

use config::Config;
use store_builder::StoreBuilder;
use graph::cheap_clone::CheapClone;
use futures::TryFutureExt;
use slog::Logger;
use graph::data::subgraph::SubgraphManifest;

lazy_static! {
    // Restart all the indexes when the indexer manager is restarted is still a new feature.
    // We don't want it to be broken when running the E2E Tests, so default option is set to False
    static ref INDEX_MANAGER_RESTART_INDEX: String = env::var("INDEX_MANAGER_RESTART_INDEX").unwrap_or(String::from("false"));
}

#[tokio::main]
async fn main() {
    // Configs Opt, IPFS Client, Link Resolver
    // Reference from: graph-node node/src/main.rs
    let opt = opt::Opt::from_args();
    let logger = logger(opt.debug);
    let ipfs_clients: Vec<_> = create_ipfs_clients(&logger, &opt.ipfs);
    let resolver = Arc::new(LinkResolver::from(ipfs_clients));

    // Config Chains
    // Reference from: ...

    // Get SubgraphManifest
    // Reference from: graph-node core/src/subgraph/instance_manager.rs
    // let manifest: SubgraphManifest<C> = {
    //     let mut manifest = SubgraphManifest::resolve_from_raw(
    //         deployment.hash.cheap_clone(),
    //         manifest,
    //         // Allow for infinite retries for subgraph definition files.
    //         &resolver.as_ref().clone().with_retries(),
    //         &logger,
    //     )
    //         .await
    //         .context("Failed to resolve subgraph from IPFS").unwrap();
    //
    //     let data_sources = load_dynamic_data_sources::<C>(
    //         store.clone(),
    //         logger.clone(),
    //         manifest.templates.clone(),
    //     )
    //         .await
    //         .context("Failed to load dynamic data sources").unwrap();
    //     // Add dynamic data sources to the subgraph
    //     manifest.data_sources.extend(data_sources);
    //     manifest
    // };


    // Get mapping
    // Reference from: core/chain/graph-ethereum/src/data_source.rs
    let mapping = mapping.resolve(&*resolver, logger).await.unwrap();
    // DataSource::from_manifest(kind, network, name, source, mapping, context)


    
    let res = init_logger(&String::from("index-manager"));
    println!("{}", res); // Print log output type

    if INDEX_MANAGER_RESTART_INDEX.to_lowercase().as_str() == "true" {
        tokio::spawn(async move {
            IndexManager::restart_all_existing_index().await;
        });
    }

    let server = IndexManager::serve("0.0.0.0:3030".to_string());
    server.wait();
}

fn create_ipfs_clients(logger: &Logger, ipfs_addresses: &Vec<String>) -> Vec<IpfsClient> {
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
            // info!(
            //     logger,
            //     "Trying IPFS node at: {}",
            //     SafeDisplay(&ipfs_address)
            // );

            let ipfs_client = match IpfsClient::new(&ipfs_address) {
                Ok(ipfs_client) => ipfs_client,
                Err(e) => {
                    // error!(
                    //     logger,
                    //     "Failed to create IPFS client for `{}`: {}",
                    //     SafeDisplay(&ipfs_address),
                    //     e
                    // );
                    panic!("Could not connect to IPFS");
                }
            };

            // Test the IPFS client by getting the version from the IPFS daemon
            let ipfs_test = ipfs_client.cheap_clone();
            let ipfs_ok_logger = logger.clone();
            let ipfs_err_logger = logger.clone();
            let ipfs_address_for_ok = ipfs_address.clone();
            let ipfs_address_for_err = ipfs_address.clone();
            graph::spawn(async move {
                ipfs_test
                    .test()
                    .map_err(move |e| {
                        // error!(
                        //     ipfs_err_logger,
                        //     "Is there an IPFS node running at \"{}\"?",
                        //     SafeDisplay(ipfs_address_for_err),
                        // );
                        panic!("Failed to connect to IPFS: {}", e);
                    })
                    .map_ok(move |_| {
                        // info!(
                        //     ipfs_ok_logger,
                        //     "Successfully connected to IPFS node at: {}",
                        //     SafeDisplay(ipfs_address_for_ok)
                        // );
                    })
                    .await
            });

            ipfs_client
        })
        .collect()
}