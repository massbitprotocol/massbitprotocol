use futures::future::join_all;
use std::collections::HashMap;
use structopt::StructOpt;

use chain_ethereum::adapter::EthereumAdapter;
use chain_ethereum::network::EthereumNetworks;
use chain_ethereum::Transport;
use massbit::blockchain::BlockchainMap;
use massbit::ipfs_client::IpfsClient;
use massbit::log::logger;
use massbit::prelude::tokio::sync::mpsc;
use massbit::prelude::{JsonRpcServer as _, *};
use massbit::util::security::SafeDisplay;
use massbit_store_postgres::IndexerStore;

use crate::config::{Config, ProviderDetails, Shard};
use crate::indexer::{
    IndexerAssignmentProvider, IndexerInstanceManager, IndexerRegistrar, LinkResolver,
};
use crate::json_rpc::JsonRpcServer;
use crate::store_builder::StoreBuilder;

mod config;
mod indexer;
mod json_rpc;
mod opt;
mod store_builder;

/// How long we will hold up node startup to get the net version and genesis
/// hash from the client. If we can't get it within that time, we'll try and
/// continue regardless.
const ETH_NET_VERSION_WAIT_TIME: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() {
    let opt = opt::Opt::from_args();

    // Set up logger
    let logger = logger(opt.debug);

    // Create a component and indexer logger factory
    let logger_factory = LoggerFactory::new(logger.clone());

    let contention_logger = logger.clone();

    let config = match Config::load(&logger, &opt.clone().into()) {
        Err(e) => {
            eprintln!("configuration error: {}", e);
            std::process::exit(1);
        }
        Ok(config) => config,
    };
    let store_builder = StoreBuilder::new(&logger, &config).await;

    // Try to create IPFS clients for each URL specified in `--ipfs`
    let ipfs_clients: Vec<_> = create_ipfs_clients(&logger, &opt.ipfs);

    // Convert the clients into a link resolver. Since we want to get past
    // possible temporary DNS failures, make the resolver retry
    let link_resolver = Arc::new(LinkResolver::from(ipfs_clients));

    let eth_networks = create_ethereum_networks(logger.clone(), config)
        .await
        .expect("Failed to parse Ethereum networks");

    // Obtain JSON-RPC server port
    let json_rpc_port = opt.admin_port;
    let node_id =
        NodeId::new(opt.node_id.clone()).expect("Node ID must contain only a-z, A-Z, 0-9, and '_'");

    let launch_services = || async move {
        let (eth_networks, idents) = connect_networks(&logger, eth_networks).await;

        let store = store_builder.store();
        let mut blockchain_map = BlockchainMap::new();
        let ethereum_chains = networks_as_chains(
            &mut blockchain_map,
            node_id.clone(),
            &eth_networks,
            &logger_factory,
        );
        let blockchain_map = Arc::new(blockchain_map);

        let indexer_instance_manager = IndexerInstanceManager::new(
            &logger_factory,
            store.cheap_clone(),
            blockchain_map.cheap_clone(),
            link_resolver.cheap_clone(),
        );

        let indexer_provider = IndexerAssignmentProvider::new(
            &logger_factory,
            link_resolver.cheap_clone(),
            indexer_instance_manager,
        );

        let indexer_registrar = Arc::new(IndexerRegistrar::new(
            &logger_factory,
            node_id.clone(),
            blockchain_map.cheap_clone(),
            link_resolver.cheap_clone(),
            store.cheap_clone(),
            Arc::new(indexer_provider),
        ));

        // Start admin JSON-RPC server.
        let json_rpc_server = JsonRpcServer::serve(
            json_rpc_port,
            indexer_registrar.clone(),
            node_id.clone(),
            logger.clone(),
        )
        .expect("failed to start JSON-RPC admin server");

        // Let the server run forever.
        std::mem::forget(json_rpc_server);
    };

    massbit::spawn(launch_services());

    // Periodically check for contention in the tokio threadpool. First spawn a
    // task that simply responds to "ping" requests. Then spawn a separate
    // thread to periodically ping it and check responsiveness.
    let (ping_send, mut ping_receive) = mpsc::channel::<crossbeam_channel::Sender<()>>(1);
    massbit::spawn(async move {
        while let Some(pong_send) = ping_receive.recv().await {
            let _ = pong_send.clone().send(());
        }
        panic!("ping sender dropped");
    });
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        let (pong_send, pong_receive) = crossbeam_channel::bounded(1);
        if futures::executor::block_on(ping_send.clone().send(pong_send)).is_err() {
            debug!(contention_logger, "Shutting down contention checker thread");
            break;
        }
        let mut timeout = Duration::from_millis(10);
        while pong_receive.recv_timeout(timeout)
            == Err(crossbeam_channel::RecvTimeoutError::Timeout)
        {
            debug!(contention_logger, "Possible contention in tokio threadpool";
                                     "timeout_ms" => timeout.as_millis(),
                                     "code" => LogCode::TokioContention);
            if timeout < Duration::from_secs(10) {
                timeout *= 10;
            } else if std::env::var_os("GRAPH_KILL_IF_UNRESPONSIVE").is_some() {
                // The node is unresponsive, kill it in hopes it will be restarted.
                std::process::abort()
            }
        }
    });

    futures::future::pending::<()>().await;
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
            info!(
                logger,
                "Trying IPFS node at: {}",
                SafeDisplay(&ipfs_address)
            );

            let ipfs_client = match IpfsClient::new(&ipfs_address) {
                Ok(ipfs_client) => ipfs_client,
                Err(e) => {
                    error!(
                        logger,
                        "Failed to create IPFS client for `{}`: {}",
                        SafeDisplay(&ipfs_address),
                        e
                    );
                    panic!("Could not connect to IPFS");
                }
            };

            // Test the IPFS client by getting the version from the IPFS daemon
            let ipfs_test = ipfs_client.cheap_clone();
            let ipfs_ok_logger = logger.clone();
            let ipfs_err_logger = logger.clone();
            let ipfs_address_for_ok = ipfs_address.clone();
            let ipfs_address_for_err = ipfs_address.clone();
            massbit::spawn(async move {
                ipfs_test
                    .test()
                    .map_err(move |e| {
                        error!(
                            ipfs_err_logger,
                            "Is there an IPFS node running at \"{}\"?",
                            SafeDisplay(ipfs_address_for_err),
                        );
                        panic!("Failed to connect to IPFS: {}", e);
                    })
                    .map_ok(move |_| {
                        info!(
                            ipfs_ok_logger,
                            "Successfully connected to IPFS node at: {}",
                            SafeDisplay(ipfs_address_for_ok)
                        );
                    })
                    .await
            });

            ipfs_client
        })
        .collect()
}

/// Parses an Ethereum connection string and returns the network name and Ethereum adapter.
async fn create_ethereum_networks(
    logger: Logger,
    config: Config,
) -> Result<EthereumNetworks, anyhow::Error> {
    let mut parsed_networks = EthereumNetworks::new();
    for (name, chain) in config.chains.chains {
        for provider in chain.providers {
            if let ProviderDetails::Web3(web3) = provider.details {
                let logger = logger.new(o!("provider" => provider.label.clone()));
                info!(
                    logger,
                    "Creating transport";
                    "url" => &web3.url,
                );

                use crate::config::Transport::*;

                let (transport_event_loop, transport) = match web3.transport {
                    Rpc => Transport::new_rpc(&web3.url, web3.headers),
                    Ipc => Transport::new_ipc(&web3.url),
                    Ws => Transport::new_ws(&web3.url),
                };

                // If we drop the event loop the transport will stop working.
                // For now it's fine to just leak it.
                std::mem::forget(transport_event_loop);

                let supports_eip_1898 = !web3.features.contains("no_eip1898");

                parsed_networks.insert(
                    name.to_string(),
                    Arc::new(
                        chain_ethereum::EthereumAdapter::new(
                            provider.label,
                            &web3.url,
                            transport,
                            supports_eip_1898,
                        )
                        .await,
                    ),
                );
            }
        }
    }
    Ok(parsed_networks)
}

/// Return the hashmap of ethereum chains and also add them to `blockchain_map`.
fn networks_as_chains(
    blockchain_map: &mut BlockchainMap,
    node_id: NodeId,
    eth_networks: &EthereumNetworks,
    logger_factory: &LoggerFactory,
) -> HashMap<String, Arc<chain_ethereum::Chain>> {
    let chains: Vec<_> = eth_networks
        .networks
        .iter()
        .map(|(network_name, eth_adapters)| {
            let chain = chain_ethereum::Chain::new(
                logger_factory.clone(),
                network_name.clone(),
                node_id.clone(),
                eth_adapters.clone(),
            );
            (network_name.clone(), Arc::new(chain))
        })
        .collect();

    for (network_name, chain) in chains.iter().cloned() {
        blockchain_map.insert::<chain_ethereum::Chain>(network_name, chain)
    }

    HashMap::from_iter(chains)
}

/// Try to connect to all the providers in `eth_networks` and get their net
/// version and genesis block. Return the same `eth_networks` and the
/// retrieved net identifiers grouped by network name. Remove all providers
/// for which trying to connect resulted in an error from the returned
/// `EthereumNetworks`, since it's likely pointless to try and connect to
/// them. If the connection attempt to a provider times out after
/// `ETH_NET_VERSION_WAIT_TIME`, keep the provider, but don't report a
/// version for it.
async fn connect_networks(
    logger: &Logger,
    mut eth_networks: EthereumNetworks,
) -> (
    EthereumNetworks,
    Vec<(String, Vec<EthereumNetworkIdentifier>)>,
) {
    // The status of a provider that we learned from connecting to it
    #[derive(PartialEq)]
    enum Status {
        Broken {
            network: String,
            provider: String,
        },
        Version {
            network: String,
            ident: EthereumNetworkIdentifier,
        },
    }

    // This has one entry for each provider, and therefore multiple entries
    // for each network
    let statuses = join_all(
        eth_networks
            .flatten()
            .into_iter()
            .map(|(network_name, eth_adapter)| (network_name, eth_adapter))
            .map(|(network, eth_adapter)| async move {
                let logger = logger.new(o!("provider" => eth_adapter.provider().to_string()));
                info!(logger, "Connecting to Ethereum to get network identifier");
                match tokio::time::timeout(ETH_NET_VERSION_WAIT_TIME, eth_adapter.net_identifiers())
                    .await
                    .map_err(Error::from)
                {
                    // An `Err` means a timeout, an `Ok(Err)` means some other error (maybe a typo
                    // on the URL)
                    Ok(Err(e)) | Err(e) => {
                        error!(logger, "Connection to provider failed. Not using this provider";
                                       "error" =>  e.to_string());
                        Status::Broken {
                            network,
                            provider: eth_adapter.provider().to_string(),
                        }
                    }
                    Ok(Ok(ident)) => {
                        info!(
                            logger,
                            "Connected to Ethereum";
                            "network_version" => &ident.net_version
                        );
                        Status::Version { network, ident }
                    }
                }
            }),
    )
    .await;

    // Group identifiers by network name
    let idents: HashMap<String, Vec<EthereumNetworkIdentifier>> =
        statuses
            .into_iter()
            .fold(HashMap::new(), |mut networks, status| {
                match status {
                    Status::Broken { network, provider } => {
                        eth_networks.remove(&network, &provider)
                    }
                    Status::Version { network, ident } => {
                        networks.entry(network.to_string()).or_default().push(ident)
                    }
                }
                networks
            });
    let idents: Vec<_> = idents.into_iter().collect();
    (eth_networks, idents)
}
