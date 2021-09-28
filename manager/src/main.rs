use indexer::{IndexerAssignmentProvider, IndexerInstanceManager, IndexerRegistrar, LinkResolver};
use massbit::ipfs_client::IpfsClient;
use massbit::prelude::{JsonRpcServer as _, *};
use structopt::StructOpt;

use crate::config::{Config, Shard};
use crate::store_builder::StoreBuilder;

mod config;
mod json_rpc;
mod opt;
mod store_builder;

#[tokio::main]
async fn main() {
    env_logger::init();

    let opt = opt::Opt::from_args();
    let config = match Config::load(&opt.clone().into()) {
        Err(e) => {
            eprintln!("configuration error: {}", e);
            std::process::exit(1);
        }
        Ok(config) => config,
    };
    let store_builder = StoreBuilder::new(&config).await;

    // Try to create IPFS clients for each URL specified in `--ipfs`
    let ipfs_clients: Vec<_> = create_ipfs_clients(&opt.ipfs);

    // Convert the clients into a link resolver. Since we want to get past
    // possible temporary DNS failures, make the resolver retry
    let link_resolver = Arc::new(LinkResolver::from(ipfs_clients));

    let indexer_registrar = Arc::new(IndexerRegistrar::new(
        link_resolver.cheap_clone(),
        store_builder.store(),
    ));

    // Obtain JSON-RPC server port
    let json_rpc_port = opt.admin_port;
    let node_id =
        NodeId::new(opt.node_id.clone()).expect("Node ID must contain only a-z, A-Z, 0-9, and '_'");

    // Start admin JSON-RPC server.
    let json_rpc_server =
        JsonRpcServer::serve(json_rpc_port, indexer_registrar.clone(), node_id.clone())
            .expect("failed to start JSON-RPC admin server");

    futures::future::pending::<()>().await;
}

fn create_ipfs_clients(ipfs_addresses: &Vec<String>) -> Vec<IpfsClient> {
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
            let ipfs_client = match IpfsClient::new(&ipfs_address) {
                Ok(ipfs_client) => ipfs_client,
                Err(e) => {
                    panic!("Could not connect to IPFS");
                }
            };

            // Test the IPFS client by getting the version from the IPFS daemon
            let ipfs_test = ipfs_client.cheap_clone();
            let ipfs_address_for_ok = ipfs_address.clone();
            let ipfs_address_for_err = ipfs_address.clone();
            massbit::spawn(async move {
                ipfs_test
                    .test()
                    .map_err(move |e| {
                        panic!("Failed to connect to IPFS: {}", e);
                    })
                    .map_ok(move |_| {})
                    .await
            });

            ipfs_client
        })
        .collect()
}
