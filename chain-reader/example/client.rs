use clap::{App, Arg};

use log::{debug, info};
use massbit::ipfs_client::IpfsClient;
use massbit_chain_solana::data_type::{decode as solana_decode, SolanaBlock, SolanaFilter};
use massbit_grpc::firehose::bstream::{
    stream_client::StreamClient, BlockRequest, BlockResponse, ChainType,
};
use std::time::Instant;

use massbit_common::NetworkType;
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};

// pub mod stream_mod {
//     tonic::include_proto!("chaindata");
// }

const URL: &str = "http://127.0.0.1:50051";
const MAX_COUNT: i32 = 3;
const SABER_STABLE_SWAP_PROGRAM: &str = "SSwpkEEcbUqx4vtoEByFjSkhKdCT862DNVb52nZg1UZ";
#[allow(dead_code)]
const SABER_ROUTER_PROGRAM: &str = "Crt7UoUR6QgrFrN7j8rmSQpUTNWNSitSwWvsWGf1qZ5t";

pub async fn print_blocks(
    mut client: StreamClient<Channel>,
    chain_type: ChainType,
    network: NetworkType,
    start_block: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Debug
    let _count = 0;
    let filter = SolanaFilter::new(vec![
        SABER_STABLE_SWAP_PROGRAM,
        // SABER_ROUTER_PROGRAM,
    ]);

    let encoded_filter = serde_json::to_vec(&filter).unwrap();
    // Not use start_block_number start_block_number yet
    let get_blocks_request = BlockRequest {
        indexer_hash: "indexer_hash".to_string(),
        start_block_number: start_block,
        chain_type: chain_type as i32,
        network,
        filter: encoded_filter,
    };
    println!("Creating Stream with {:?}", &get_blocks_request);
    let mut stream = Some(
        client
            .blocks(Request::new(get_blocks_request))
            .await?
            .into_inner(),
    );

    println!("Waitting for data...");
    while let Some(data) = stream.as_mut().unwrap().message().await? {
        let mut data = data as BlockResponse;
        match chain_type {
            ChainType::Solana => {
                let now = Instant::now();
                let blocks: Vec<SolanaBlock> = solana_decode(&mut data.payload).unwrap();
                // Decode
                // let block = convert_solana_encoded_block_to_solana_block(encoded_block);
                // let mut print_flag = true;
                info!("SOLANA: Recieved {} BLOCK.", blocks.len());
                for block in blocks {
                    info!(
                        "SOLANA: Recieved {} TRANSACTIONS in Block slot: {:?}",
                        &block.block.transactions.len(),
                        block.block_number
                    );
                }

                let elapsed = now.elapsed();
                debug!("Elapsed processing solana block: {:.2?}", elapsed);
            }
            _ => {}
        }
    }
    Ok(())
}

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

            // let ipfs_test = ipfs_client.clone();
            // Hughie: comment out the check for connection because there's an error with tokio spawm runtime
            // We can use tokio02 spawn custom function to fix this problem

            // #[allow(unused_must_use)]
            // tokio::spawn(async move {
            //     ipfs_test
            //         .test()
            //         .map_err(move |e| {
            //             panic!("[Ipfs Client] Failed to connect to IPFS: {}", e);
            //         })
            //         .map_ok(move |_| {
            //             log::info!("[Ipfs Client] Successfully connected to IPFS node");
            //         }).await;
            // });
            ipfs_client
        })
        .collect()
}

// async fn get_data_source(
//     file_hash: &String,
// ) -> Result<Vec<DataSource>, SubgraphAssignmentProviderError> {
//     let logger = logger(false);
//     let ipfs_addresses = vec![String::from("0.0.0.0:5001")];
//     let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;
//
//     // let mut resolver = TextResolver::default();
//     let file_bytes = ipfs_clients[0]
//         .cat_all(file_hash.to_string(), Duration::from_secs(10))
//         .compat()
//         .await
//         .unwrap()
//         .to_vec();
//
//     // Get raw manifest
//     let file = String::from_utf8(file_bytes).unwrap();
//     println!("File: {}", file);
//
//     let raw: serde_yaml::Value = serde_yaml::from_str(&file).unwrap();
//
//     let mut raw_manifest = match raw {
//         serde_yaml::Value::Mapping(m) => m,
//         _ => panic!("Wrong type raw_manifest"),
//     };
//
//     // Inject the IPFS hash as the ID of the subgraph into the definition.
//     let id = "deployment_hash";
//     raw_manifest.insert(
//         serde_yaml::Value::from("id"),
//         serde_yaml::Value::from(id.to_string()),
//     );
//
//     // Parse the YAML data into an UnresolvedSubgraphManifest
//     let value: Value = raw_manifest.into();
//     let unresolved: UnresolvedSubgraphManifest<Chain> = serde_yaml::from_value(value).unwrap();
//     let resolver = Arc::new(LinkResolver::from(ipfs_clients));
//
//     //debug!("Features {:?}", unresolved.features);
//     let manifest = unresolved
//         .resolve(&*resolver, &logger)
//         .compat()
//         .await
//         .map_err(SubgraphAssignmentProviderError::ResolveError)?;
//
//     println!("data_sources: {:#?}", &manifest.data_sources);
//
//     Ok(manifest.data_sources)
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    env_logger::init();
    info!("Waiting for chain-reader");

    let matches = App::new("Client")
        .version("1.0")
        .about("Client for test chain-reader")
        .arg(
            Arg::with_name("type")
                .short("c")
                .long("chain-type")
                .value_name("type")
                .help("Sets chain type")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("start-block")
                .short("s")
                .long("start-block")
                .value_name("start-block")
                .help("Sets chain type")
                .takes_value(true),
        )
        .get_matches();

    let chain_type = matches.value_of("type").unwrap_or("solana");
    let start_block: Option<u64> = matches.value_of("start-block").map(|start_block| {
        println!("start_block: {:?}", start_block);
        let start_block: u64 = start_block.parse().unwrap();
        start_block
    });

    let client = StreamClient::connect(URL).await.unwrap();
    println!("Match {:?}", matches);
    match chain_type {
        "solana" => {
            info!("Run client: {}", chain_type);
            print_blocks(
                client,
                ChainType::Solana,
                "mainnet".to_string(),
                start_block,
            )
            .await?;
        }
        _ => {
            info!("Run client: {}", chain_type);
            print_blocks(client, ChainType::Solana, "matic".to_string(), start_block).await?;
        }
    };

    Ok(())
}
