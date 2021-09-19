use log::{debug, info, warn};
#[allow(unused_imports)]
use tonic::{
    Request,
    Response, Status, transport::{Channel, Server},
};

use massbit_chain_ethereum::data_type::{decode as ethereum_decode, EthereumBlock, get_events};

use diesel::{insert_into, RunQueryDsl};
use crate::stream_mod::{
    ChainType, DataType, GenericDataProto, GetBlocksRequest, streamout_client::StreamoutClient,
};

pub mod models;

use models::{MaticBlock, MaticTransaction};
use crate::schema::{matic_block::{self, *}, matic_transaction::{self, *}};
use crate::establish_connection;
use std::time::Instant;

pub async fn process_ethereum_block(mut client: StreamoutClient<Channel>, network: String)
    ->  Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let get_blocks_request = GetBlocksRequest {
        start_block_number: 0,
        end_block_number: 1,
        chain_type: ChainType::Ethereum as i32,
        network,
    };
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();
    let conn = establish_connection();
    log::info!("Starting read blocks from stream...");
    let conn = establish_connection();
    while let Some(data) = stream.message().await? {
        let mut data = data as GenericDataProto;
        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                let start = Instant::now();
                let block: EthereumBlock = ethereum_decode(&mut data.payload).unwrap();
                let matic_block = MaticBlock::from(&block.block);
                insert_into(matic_block::table)
                    .values(&matic_block)
                    .execute(&conn);
                let transactions = block.block.transactions.iter().map(|tran|{
                    let mut transaction = MaticTransaction::from(tran);
                    transaction.timestamp = matic_block.timestamp;
                    transaction
                }).collect::<Vec<MaticTransaction>>();
                let result = insert_into(matic_transaction::table)
                    .values(&transactions)
                    .execute(&conn);
                info!(
                        "Dump Ethereum BLOCK: {} with {} transactions in {:?}. Result {:?}",
                        &block.block.number.unwrap().as_u64(),
                        matic_block.transaction_number,
                        start.elapsed(),
                        &result
                );
            }
            _ => {
                warn!("Not support this type in Ethereum");
            }
        };
    }
    Ok(())
}
