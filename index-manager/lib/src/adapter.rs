use crate::ipfs::read_config_file;
use crate::types::IndexConfig;
use adapter::core::AdapterManager;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

const QUICKSWAP_PATH: &str = r#"/home/viettai/Massbit/QuickSwap-subgraph"#;
const WASM_FILE: &str = r#"build/Factory/Factory.wasm"#;
const MANIFEST: &str = r#"subgraph.yaml"#;
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}
use log::{debug, info, warn, Level};
//use massbit_chain_ethereum::data_type::{decode as ethereum_decode, EthereumBlock};
use massbit_chain_solana::data_type::{
    convert_solana_encoded_block_to_solana_block, decode as solana_decode, SolanaEncodedBlock,
    SolanaLogMessages, SolanaTransaction,
};
use massbit_chain_substrate::data_type::{SubstrateBlock, SubstrateEventRecord};
use stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};

use massbit_chain_substrate::data_type::{decode, get_extrinsics_from_block};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

lazy_static::lazy_static! {
    static ref CHAIN_READER_URL: String =
        std::env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
}
const URL: &str = "http://127.0.0.1:50051";

pub async fn adapter_init(index_config: &IndexConfig) -> Result<(), Box<dyn Error>> {
    // Chain Reader Client Configuration to subscribe and get latest block from Chain Reader Server
    let config_value = read_config_file(&index_config.config);
    //let config_path = PathBuf::from(format!("{}/{}", QUICKSWAP_PATH, MANIFEST).as_str());
    //let config_value = read_config_file(&config_path);
    log::info!("Load library from {:?}", &index_config.mapping);
    /*
    let runtime_path = PathBuf::from(format!("{}/{}", QUICKSWAP_PATH, WASM_FILE).as_str());
    let mut adapter = AdapterManager::new();
    adapter
        .init(
            &index_config.identifier.name_with_hash,
            &config_value,
            //&runtime_path,
            &index_config.mapping,
        )
        .await;

     */
    let client = StreamoutClient::connect(CHAIN_READER_URL.clone())
        .await
        .unwrap();
    //print_blocks(client, ChainType::Ethereum).await;
    print_blocks(client, ChainType::Solana).await;
    Ok(())
}

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
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();

    while let Some(data) = stream.message().await? {
        let mut data = data as GenericDataProto;
        println!(
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
                        //println!("{:?}", data);
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
                            let rc_transaction = Arc::new(transaction.clone());

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
            /*
            ChainType::Ethereum => match DataType::from_i32(data.data_type) {
                Some(DataType::Block) => {
                    let block: EthereumBlock = ethereum_decode(&mut data.payload).unwrap();
                    info!(
                        "Recieved ETHREUM BLOCK with Block number: {}",
                        &block.block.number.unwrap().as_u64()
                    );
                }
                _ => {
                    warn!("Not support this type in Ethereum");
                }
            },
             */
            _ => {
                warn!("Not support this package chain-type");
            }
        }
    }

    Ok(())
}
