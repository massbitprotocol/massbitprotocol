use std::time::Instant;
use tonic::Request;
use lazy_static::lazy_static;
use std::{env, fs};

// Massbit dependencies
use plugin::PluginManager;
use crate::manifest::get_chain_type;
use massbit_chain_substrate::data_type::{decode, SubstrateBlock, get_extrinsics_from_block, SubstrateEventRecord};
use massbit_chain_solana::data_type::{decode as solana_decode, SolanaEncodedBlock, convert_solana_encoded_block_to_solana_block, SolanaTransaction, SolanaLogMessages};
use crate::types::stream_mod::{GetBlocksRequest, GenericDataProto, ChainType, DataType, streamout_client::StreamoutClient};
use serde_yaml::Value;
use index_store::core::IndexStore;
use std::path::PathBuf;

lazy_static! {
    static ref CHAIN_READER_URL: String =
        env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
}

pub async fn chain_reader_client_start(config: &Value, mapping: &PathBuf) {
    let mut store = IndexStore::new(DATABASE_CONNECTION_STRING.as_str());
    let mut client = StreamoutClient::connect(CHAIN_READER_URL.clone())
        .await
        .unwrap();
    let chain_type = get_chain_type(&config);
    let get_blocks_request = GetBlocksRequest {
        start_block_number: 0,
        end_block_number: 1,
        chain_type: chain_type as i32,
    };
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await.unwrap()
        .into_inner();
    // Subscribe new blocks
    log::info!("[Index Manager Helper] Start processing block");
    while let Some(data) = stream.message().await.unwrap() {
        let now = Instant::now();
        let mut data = data as GenericDataProto;
        log::info!("[Index Manager Helper] Received chain: {:?}, data block = {:?}, hash = {:?}, data type = {:?}",
                   ChainType::from_i32(data.chain_type).unwrap(),
                   data.block_number,
                   data.block_hash,
                   DataType::from_i32(data.data_type).unwrap());
        // TODO: Refactor this or this will be called every time a new block comes
        let mut plugins = PluginManager::new(&mut store);
        unsafe {
            plugins.load("1234", mapping.clone()).unwrap();
        }
        match chain_type {
            ChainType::Substrate => {
                match DataType::from_i32(data.data_type) {
                    Some(DataType::Block) => {
                        let block: SubstrateBlock = decode(&mut data.payload).unwrap();
                        println!("Received BLOCK: {:?}", &block.block.header.number);
                        let extrinsics = get_extrinsics_from_block(&block);
                        for extrinsic in extrinsics {
                            println!("Received EXTRINSIC: {:?}", extrinsic);
                            plugins.handle_substrate_extrinsic("1234", &extrinsic);
                        }
                        plugins.handle_substrate_block("1234", &block);
                    }
                    Some(DataType::Event) => {
                        let event: SubstrateEventRecord = decode(&mut data.payload).unwrap();
                        println!("Received Event: {:?}", event);
                        plugins.handle_substrate_event("1234", &event);
                    }
                    _ => {
                        println!("Not support data type: {:?}", &data.data_type);
                    }
                } // End of Substrate i32 data
            } // End of Substrate type
            ChainType::Solana => {
                match DataType::from_i32(data.data_type) {
                    Some(DataType::Block) => {
                        let encoded_block: SolanaEncodedBlock = solana_decode(&mut data.payload).unwrap();
                        let block = convert_solana_encoded_block_to_solana_block(encoded_block); // Decoding
                        println!("Received SOLANA BLOCK with block height: {:?}, hash: {:?}", &block.block.block_height.unwrap(), &block.block.blockhash);
                        plugins.handle_solana_block("1234", &block);
                        let mut print_flag = true;
                        for origin_transaction in block.clone().block.transactions {
                            let origin_log_messages = origin_transaction.meta.clone().unwrap().log_messages;
                            let transaction = SolanaTransaction {
                                block_number: ((&block).block.block_height.unwrap() as u32),
                                transaction: origin_transaction.clone(),
                                log_messages: origin_log_messages.clone(),
                                success: false
                            };

                            let log_messages = SolanaLogMessages {
                                block_number: ((&block).block.block_height.unwrap() as u32),
                                log_messages: origin_log_messages.clone(),
                                transaction: origin_transaction.clone(),
                            };
                            if print_flag {
                                println!("Recieved SOLANA TRANSACTION with Block number: {:?}, transaction: {:?}", &transaction.block_number, &transaction.transaction.transaction.signatures);
                                println!("Recieved SOLANA LOG_MESSAGES with Block number: {:?}, log_messages: {:?}", &log_messages.block_number, &log_messages.log_messages.clone().unwrap().get(0));
                                print_flag = false;
                            }
                            plugins.handle_solana_transaction("1234", &transaction);
                            plugins.handle_solana_log_messages("1234", &log_messages);
                        }
                    },
                    _ => {
                        println!("Not support type in Solana");
                    }
                } // End of Solana i32 data
            }, // End of Solana type
            _ => {
                println!("Not support this package chain-type");
            }
        }
        let elapsed = now.elapsed();
        println!("Elapsed processing block: {:.2?}", elapsed);
    }
}