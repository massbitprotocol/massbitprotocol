use crate::stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
use log::{debug, error, info, warn, Level};
use massbit_chain_solana::data_type::{
    convert_solana_encoded_block_to_solana_block, decode as solana_decode, decode_encoded_block,
    SolanaEncodedBlock, SolanaLogMessages, SolanaTransaction,
};
use massbit_chain_substrate::data_type::{SubstrateBlock, SubstrateEventRecord};
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};

pub mod stream_mod {
    tonic::include_proto!("chaindata");
}
use massbit_chain_substrate::data_type::{decode, get_extrinsics_from_block};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

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
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();

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
                            //info!("Received EXTRINSIC: {:?}", extrinsic);
                            let string_extrinsic = format!("Received EXTRINSIC:{:?}", extrinsic);
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
                        let block = decode_encoded_block(encoded_block.block);

                        // Get transactions
                        let transactions = block.transactions;

                        // Check each transaction to find serum data
                        for origin_transaction_with_status_meta in transactions {
                            let serum_dex_key = "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin";
                            let mut check_key = false;
                            //let decode_transaction = origin_transaction_with_status_meta.transaction.decode().unwrap();
                            for acc_key in &origin_transaction_with_status_meta
                                .transaction
                                .message
                                .account_keys
                            {
                                if acc_key.to_string() == serum_dex_key {
                                    check_key = true;
                                    break;
                                }
                            }

                            if check_key {
                                // Print serum data
                                info!("Serum trans: {:#?}", origin_transaction_with_status_meta);
                                //loop{};
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
            _ => {
                warn!("Not support this package chain-type");
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    env_logger::init();
    info!("Waiting for chain-reader");

    // tokio::spawn(async move {

    let client = StreamoutClient::connect(URL).await.unwrap();
    print_blocks(client, ChainType::Solana).await;
    // });

    // let client = StreamoutClient::connect(URL).await.unwrap();
    // print_blocks(client, ChainType::Substrate).await?;

    loop {}
    Ok(())
}
