use clap::{App, Arg};

use graph::data::subgraph::UnresolvedSubgraphManifest;
use graph::ipfs_client::IpfsClient;
use graph_core::LinkResolver;
use log::{debug, info, warn};
use massbit::firehose::bstream::{
    stream_client::StreamClient, BlockResponse, BlocksRequest, ChainType,
};
use massbit_chain_ethereum::data_type::{decode as ethereum_decode, get_events, EthereumBlock};
use massbit_chain_solana::data_type::{
    convert_solana_encoded_block_to_solana_block, decode as solana_decode, SolanaEncodedBlock,
    SolanaLogMessages, SolanaTransaction,
};
use massbit_chain_substrate::data_type::{SubstrateBlock, SubstrateEventRecord};

use graph::data::subgraph::SubgraphAssignmentProviderError;
use graph::log::logger;
use graph_chain_ethereum::{Chain, DataSource};

use massbit_chain_substrate::data_type::{decode, get_extrinsics_from_block};

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_compat_02::FutureExt;

use serde_yaml::Value;

use massbit::blockchain::{Blockchain, TriggerFilter};
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

pub async fn print_blocks(
    mut client: StreamClient<Channel>,
    chain_type: ChainType,
    network: NetworkType,
    start_block: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Debug
    let mut count = 0;
    let filter =
        <chain_ethereum::Chain as Blockchain>::TriggerFilter::from_data_sources(vec![].iter());
    let encoded_filter = serde_json::to_vec(&filter).unwrap();
    // Not use start_block_number start_block_number yet
    let get_blocks_request = BlocksRequest {
        start_block_number: start_block,
        chain_type: chain_type as i32,
        network,
        filter: encoded_filter,
    };
    println!("Creating Stream ...");
    let mut stream = Some(
        client
            .blocks(Request::new(get_blocks_request))
            .await?
            .into_inner(),
    );

    let mut file_hash = "".to_string();
    let mut data_sources: Vec<DataSource> = vec![];
    // if chain_type == ChainType::Ethereum {
    //     // For ethereum only
    //     file_hash = "/ipfs/QmVVrXLPKJYiXQqmR5LVmPTJBbYEQp4vgwve3hqXroHDp5".to_string();
    //     data_sources = get_data_source(&file_hash).await.unwrap();
    //     // End For ethereum only
    // }

    println!("Waitting for data...");
    while let Some(data) = stream.as_mut().unwrap().message().await? {
        let mut data = data as BlockResponse;
        println!(
            "Received chain: {:?}, data block = {:?}, hash = {:?}",
            ChainType::from_i32(data.chain_type).unwrap(),
            data.block_number,
            data.block_hash,
        );
        match chain_type {
            ChainType::Substrate => {
                let now = Instant::now();
                let block: SubstrateBlock = decode(&mut data.payload).unwrap();
                info!("Received BLOCK: {:?}", &block.block.header.number);
                let extrinsics = get_extrinsics_from_block(&block);
                for extrinsic in extrinsics {
                    //info!("Recieved EXTRINSIC: {:?}", extrinsic);
                    let string_extrinsic = format!("Recieved EXTRINSIC:{:?}", extrinsic);
                    info!("{}", string_extrinsic);
                }
                let elapsed = now.elapsed();
                debug!("Elapsed processing solana block: {:.2?}", elapsed);
            }
            ChainType::Solana => {
                let now = Instant::now();
                let encoded_block: SolanaEncodedBlock = solana_decode(&mut data.payload).unwrap();
                // Decode
                let block = convert_solana_encoded_block_to_solana_block(encoded_block);
                let mut print_flag = true;
                for origin_transaction in block.clone().block.transactions {
                    let log_messages = origin_transaction
                        .clone()
                        .meta
                        .unwrap()
                        .log_messages
                        .clone();
                    let transaction = SolanaTransaction {
                        block_number: ((&block).block.block_height.unwrap() as u32),
                        transaction: origin_transaction.clone(),
                        log_messages: log_messages.clone(),
                        success: false,
                    };
                    let log_messages = SolanaLogMessages {
                        block_number: ((&block).block.block_height.unwrap() as u32),
                        log_messages: log_messages.clone(),
                        transaction: origin_transaction.clone(),
                    };

                    // Print first data only bc it too many.
                    if print_flag {
                        info!("Recieved SOLANA TRANSACTION with Block number: {:?}, trainsation: {:?}", &transaction.block_number, &transaction.transaction.transaction.signatures);
                        info!("Recieved SOLANA LOG_MESSAGES with Block number: {:?}, log_messages: {:?}", &log_messages.block_number, &log_messages.log_messages.unwrap().get(0));

                        print_flag = false;
                    }
                }
                let elapsed = now.elapsed();
                debug!("Elapsed processing solana block: {:.2?}", elapsed);
            }
            ChainType::Ethereum => {
                let block: EthereumBlock = ethereum_decode(&mut data.payload).unwrap();
                info!(
                    "Recieved ETHREUM BLOCK with Block number: {}",
                    &block.block.number.unwrap().as_u64()
                );

                count += 1;
                if count >= MAX_COUNT {
                    break;
                }

                // for data_source in &data_sources {
                //     //println!("data_source: {:#?}", &data_source);
                //     let events = get_events(&block, data_source);
                //
                //     // for event in events {
                //     //     println!("Ethereum Event address: {:?}", &event.event.address);
                //     // }
                // }
            }
        }
    }
    //drop(stream);
    //stream = None;
    loop {}

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

    let chain_type = matches.value_of("type").unwrap_or("ethereum");
    let start_block: Option<u64> = matches
        .value_of("start-block")
        .map(|start_block| start_block.parse().unwrap());
    let client = StreamClient::connect(URL).await.unwrap();
    println!("Match {:?}", matches);
    match chain_type {
        "substrate" => {
            info!("Run client: {}", chain_type);
            print_blocks(
                client,
                ChainType::Substrate,
                "mainnet".to_string(),
                start_block,
            )
            .await?;
        }
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
            print_blocks(
                client,
                ChainType::Ethereum,
                "matic".to_string(),
                start_block,
            )
            .await?;
        }
    };

    Ok(())
}
