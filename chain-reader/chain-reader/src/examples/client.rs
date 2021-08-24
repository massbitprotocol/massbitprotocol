use clap::{App, Arg};

use crate::stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
use graph::data::subgraph::UnresolvedSubgraphManifest;
use graph::ipfs_client::IpfsClient;
use graph_core::LinkResolver;
use log::{debug, info, warn};
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

#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};

pub mod stream_mod {
    tonic::include_proto!("chaindata");
}

const URL: &str = "http://127.0.0.1:50051";

pub async fn print_blocks(
    mut client: StreamoutClient<Channel>,
    chain_type: ChainType,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Not use start_block_number start_block_number yet
    let get_blocks_request = GetBlocksRequest {
        start_block_number: 0,
        end_block_number: 1,
        chain_type: chain_type as i32,
    };
    println!("Creating Stream ...");
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();
    println!("Waitting for data...");
    while let Some(data) = stream.message().await? {
        let mut data = data as GenericDataProto;
        info!(
            "Received chain: {:?}, data block = {:?}, hash = {:?}, data type = {:?}",
            ChainType::from_i32(data.chain_type).unwrap(),
            data.block_number,
            data.block_hash,
            DataType::from_i32(data.data_type).unwrap()
        );
        match chain_type {
            ChainType::Substrate => {
                let now = Instant::now();
                match DataType::from_i32(data.data_type) {
                    Some(DataType::Block) => {
                        let block: SubstrateBlock = decode(&mut data.payload).unwrap();
                        info!("Received BLOCK: {:?}", &block.block.header.number);
                        let extrinsics = get_extrinsics_from_block(&block);
                        for extrinsic in extrinsics {
                            //info!("Recieved EXTRINSIC: {:?}", extrinsic);
                            let string_extrinsic = format!("Recieved EXTRINSIC:{:?}", extrinsic);
                            info!("{}", string_extrinsic);
                        }
                    }
                    Some(DataType::Event) => {
                        let event: Vec<SubstrateEventRecord> = decode(&mut data.payload).unwrap();
                        info!("Received Event: {:?}", event);
                    }

                    _ => {
                        warn!("Not support data type: {:?}", &data.data_type);
                    }
                }
                let elapsed = now.elapsed();
                debug!("Elapsed processing solana block: {:.2?}", elapsed);
            }
            ChainType::Solana => {
                let now = Instant::now();
                match DataType::from_i32(data.data_type) {
                    Some(DataType::Block) => {
                        let encoded_block: SolanaEncodedBlock =
                            solana_decode(&mut data.payload).unwrap();
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
                    }
                    _ => {
                        warn!("Not support this type in Solana");
                    }
                }
                let elapsed = now.elapsed();
                debug!("Elapsed processing solana block: {:.2?}", elapsed);
            }
            ChainType::Ethereum => match DataType::from_i32(data.data_type) {
                Some(DataType::Block) => {
                    let block: EthereumBlock = ethereum_decode(&mut data.payload).unwrap();
                    info!(
                        "Recieved ETHREUM BLOCK with Block number: {}",
                        &block.block.number.unwrap().as_u64()
                    );
                    let file_hash =
                        "/ipfs/QmVVrXLPKJYiXQqmR5LVmPTJBbYEQp4vgwve3hqXroHDp5".to_string();
                    let data_sources: Vec<DataSource> = get_data_source(&file_hash).await.unwrap();
                    for data_source in data_sources {
                        println!("data_source: {:#?}", &data_source);
                        let events = get_events(&block, data_source);

                        for event in events {
                            println!("Ethereum Event address: {:?}", &event.event.address);
                        }
                    }
                }
                _ => {
                    warn!("Not support this type in Ethereum");
                }
            },
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

async fn get_data_source(
    file_hash: &String,
) -> Result<Vec<DataSource>, SubgraphAssignmentProviderError> {
    let logger = logger(true);
    let ipfs_addresses = vec![String::from("0.0.0.0:5001")];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;

    // let mut resolver = TextResolver::default();
    let file_bytes = ipfs_clients[0]
        .cat_all(file_hash.to_string(), Duration::from_secs(10))
        .compat()
        .await
        .unwrap()
        .to_vec();

    // Get raw manifest
    let file = String::from_utf8(file_bytes)
        //.map_err(|_| SubgraphAssignmentProviderError::ResolveError)
        .unwrap();

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

    //println!("raw_manifest: {:#?}", &raw_manifest);
    // Parse the YAML data into an UnresolvedSubgraphManifest
    let value: Value = raw_manifest.into();
    //println!("value: {:#?}", &value);
    let unresolved: UnresolvedSubgraphManifest<Chain> = serde_yaml::from_value(value).unwrap();
    let resolver = Arc::new(LinkResolver::from(ipfs_clients));

    debug!("Features {:?}", unresolved.features);
    let manifest = unresolved
        .resolve(&*resolver, &logger)
        .compat()
        .await
        .map_err(SubgraphAssignmentProviderError::ResolveError)?;

    println!("data_sources: {:#?}", &manifest.data_sources);

    Ok(manifest.data_sources)
}

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
        .get_matches();

    let chain_type = matches.value_of("type").unwrap_or("ethereum");
    let client = StreamoutClient::connect(URL).await.unwrap();
    println!("Match {:?}", matches);
    match chain_type {
        "substrate" => {
            info!("Run client: {}", chain_type);
            print_blocks(client, ChainType::Substrate).await?;
        }
        "solana" => {
            info!("Run client: {}", chain_type);
            print_blocks(client, ChainType::Solana).await?;
        }
        _ => {
            info!("Run client: {}", chain_type);
            print_blocks(client, ChainType::Ethereum).await?;
        }
    };

    Ok(())
}
