use crate::stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
    HelloRequest,
};
use massbit_chain_solana::data_type::{
    convert_solana_encoded_block_to_solana_block, decode as solana_decode, SolanaBlock,
    SolanaEncodedBlock, SolanaLogMessages, SolanaTransaction,
};
use massbit_chain_substrate::data_type::{
    decode_transactions, SubstrateBlock, SubstrateEventRecord, SubstrateHeader,
    SubstrateUncheckedExtrinsic,
};
use std::error::Error;
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};

use codec::{Decode, Encode};
use node_template_runtime::Event;
use sp_core::{sr25519, H256 as Hash};
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}
use massbit_chain_substrate::data_type::{decode, get_extrinsics_from_block};
use std::sync::Arc;

type EventRecord = system::EventRecord<Event, Hash>;

const URL: &str = "http://127.0.0.1:50051";

pub async fn print_blocks(
    mut client: StreamoutClient<Channel>,
    chain_type: ChainType,
) -> Result<(), Box<dyn Error>> {
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
                match DataType::from_i32(data.data_type) {
                    Some(DataType::Block) => {
                        let block: SubstrateBlock = decode(&mut data.payload).unwrap();
                        println!("Received BLOCK: {:?}", &block.block.header.number);
                        let extrinsics = get_extrinsics_from_block(&block);
                        for extrinsic in extrinsics {
                            //println!("Recieved EXTRINSIC: {:?}", extrinsic);
                            let string_extrinsic = format!("Recieved EXTRINSIC:{:?}", extrinsic);
                            println!("{}", string_extrinsic);
                        }
                    }
                    Some(DataType::Event) => {
                        let event: Vec<SubstrateEventRecord> = decode(&mut data.payload).unwrap();
                        println!("Received Event: {:?}", event);
                    },

                    _ => {
                        println!("Not support data type: {:?}", &data.data_type);
                    }
                }
            }
            ChainType::Solana => {
                match DataType::from_i32(data.data_type) {
                    Some(DataType::Block) => {
                        let encoded_block: SolanaEncodedBlock =
                            solana_decode(&mut data.payload).unwrap();
                        // Decode
                        let block = convert_solana_encoded_block_to_solana_block(encoded_block);

                        println!(
                            "Recieved SOLANA BLOCK with block height: {:?}, hash: {:?}",
                            &block.block.block_height.unwrap(),
                            &block.block.transactions
                        );

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
                                block: block.clone(),
                                log_messages: log_messages.clone(),
                                success: false,
                            };
                            println!("Recieved SOLANA TRANSACTION with Block number: {:?}, trainsation: {:?}", &transaction.block_number, &transaction.transaction);

                            let log_messages = SolanaLogMessages {
                                block_number: ((&block).block.block_height.unwrap() as u32),
                                log_messages: log_messages.clone(),
                                transaction: transaction.clone(),
                                block: block.clone(),
                            };
                            println!("Recieved SOLANA LOG_MESSAGES with Block number: {:?}, log_messages: {:?}", &transaction.block_number, &transaction.log_messages);
                        }
                    }
                    _ => {
                        println!("Not support this type in Solana");
                    }
                }
            }
            _ => {
                println!("Not support this package chain-type");
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Waiting for chain-reader");

    tokio::spawn(async move {
        let mut client = StreamoutClient::connect(URL).await.unwrap();
        print_blocks(client, ChainType::Solana).await;
    });

    let mut client = StreamoutClient::connect(URL).await.unwrap();
    print_blocks(client, ChainType::Substrate).await;

    Ok(())
}
