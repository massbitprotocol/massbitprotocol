pub mod handler;
pub mod metrics;
pub mod models;
use diesel::{self, RunQueryDsl};
pub use handler::EthereumHandlerManager;
use lazy_static::lazy_static;
use log::{info, warn};
use massbit_chain_ethereum::data_type::{
    decode as ethereum_decode, EthereumBlock as EthereumChainBlock,
};
use massbit_common::prelude::tokio::time::{sleep, timeout, Duration};
use massbit_common::NetworkType;
use models::{EthereumBlock, EthereumTransaction};
use std::time::Instant;
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status, Streaming,
};

use crate::stream_mod::{streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto};
use crate::{
    establish_connection, get_block_number, try_create_stream, GET_BLOCK_TIMEOUT_SEC,
    GET_STREAM_TIMEOUT_SEC,
};

use crate::ethereum::handler::{create_ethereum_handler_manager, EthereumHandler};
use crate::postgres_adapter::PostgresAdapter;
use crate::schema::*;
use crate::storage_adapter::StorageAdapter;
use graph::prelude::Value;
use massbit_common::prelude::diesel::pg::upsert::excluded;
use massbit_common::prelude::diesel::result::Error;
use massbit_common::prelude::diesel::ExpressionMethods;
use std::sync::Arc;
use tower::timeout::Timeout;

lazy_static! {
    pub static ref CHAIN: String = String::from("ethereum");
}
const START_ETHEREUM_BLOCK: u64 = 15_000_000_u64;
const DEFAULT_NETWORK: &str = "matic";

pub async fn process_ethereum_stream(
    client: &mut StreamoutClient<Timeout<Channel>>,
    storage_adapter: Arc<PostgresAdapter>,
    network: Option<NetworkType>,
    block: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let handler_manager = create_ethereum_handler_manager(&network, storage_adapter);
    //Todo: remove this simple connection
    let conn = establish_connection();
    let current_state = get_block_number(
        &conn,
        CHAIN.clone(),
        network.clone().unwrap_or(String::from(DEFAULT_NETWORK)),
    );
    let start_block = match current_state {
        None => {
            if block > 0 {
                block
            } else {
                START_ETHEREUM_BLOCK
            }
        }
        Some(state) => state.got_block as u64 + 1,
    };
    let mut opt_stream: Option<Streaming<GenericDataProto>> = None;
    loop {
        match opt_stream {
            None => {
                opt_stream =
                    try_create_stream(client, ChainType::Ethereum, start_block.clone(), &network)
                        .await;
                if opt_stream.is_none() {
                    //Sleep for a while and reconnect
                    sleep(Duration::from_secs(GET_STREAM_TIMEOUT_SEC)).await;
                }
            }
            Some(ref mut stream) => {
                let response =
                    timeout(Duration::from_secs(GET_BLOCK_TIMEOUT_SEC), stream.message()).await;
                match response {
                    Ok(Ok(res)) => {
                        if let Some(mut data) = res {
                            match DataType::from_i32(data.data_type) {
                                Some(DataType::Block) => {
                                    let start = Instant::now();
                                    let block: EthereumChainBlock =
                                        ethereum_decode(&mut data.payload).unwrap();
                                    let block_number = match block.block.number {
                                        None => 0_i64,
                                        Some(val) => val.as_u64() as i64,
                                    };
                                    handler_manager.handle_ext_block(&block);
                                    match diesel::insert_into(network_state::table)
                                        .values((
                                            network_state::chain.eq(CHAIN.clone()),
                                            network_state::network.eq(network
                                                .clone()
                                                .unwrap_or(DEFAULT_NETWORK.to_string())),
                                            network_state::got_block.eq(block_number.clone()),
                                        ))
                                        .on_conflict((network_state::chain, network_state::network))
                                        .do_update()
                                        .set(
                                            network_state::got_block
                                                .eq(excluded(network_state::got_block)),
                                        )
                                        .execute(&conn)
                                    {
                                        Ok(_) => {}
                                        Err(err) => log::error!("{:?}", &err),
                                    };
                                    log::info!("Block {} with {} receipts and {} transactions is processed in {:?}",
                                        block_number, block.receipts.len(), block.block.transactions.len(), start.elapsed());
                                }
                                _ => {
                                    warn!("Not support this type in Ethereum");
                                }
                            };
                        }
                    }
                    _ => {
                        log::info!(
                            "Error while get message from reader stream {:?}. Recreate stream",
                            &response
                        );
                        opt_stream = None;
                    }
                }
            }
        };
    }
    Ok(())
}

// pub async fn _process_ethereum_stream(client: &mut StreamoutClient<Timeout<Channel>>,
//                                     storage_adapter: &dyn StorageAdapter,
//                                     network: &Option<NetworkType>,
//                                     block: u64)
//     ->  Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
//     //Todo: remove this simpe connection
//     let conn = establish_connection();
//     let current_state = get_block_number(&conn, CHAIN.clone(), network.clone().unwrap_or(String::from(DEFAULT_NETWORK)));
//     let start_block = match current_state {
//         None =>
//             if block > 0 {
//                 block
//             } else {
//                 START_ETHEREUM_BLOCK
//             },
//         Some(state) => state.got_block as u64 + 1
//     };
//     let mut opt_stream: Option<Streaming<GenericDataProto>> = None;
//     loop {
//         match opt_stream {
//             None => {
//                 opt_stream = try_create_stream(
//                     client,
//                     ChainType::Ethereum,
//                     start_block,
//                     &network,
//                 )
//                     .await;
//                 if opt_stream.is_none() {
//                     //Sleep for a while and reconnect
//                     sleep(Duration::from_secs(GET_STREAM_TIMEOUT_SEC)).await;
//                 }
//             }
//             Some(ref mut stream) => {
//                 let response = timeout(
//                     Duration::from_secs(GET_BLOCK_TIMEOUT_SEC),
//                     stream.message(),
//                 )
//                     .await;
//                 match response {
//                     Ok(Ok(res)) => {
//                         if let Some(mut data) = res {
//                             match DataType::from_i32(data.data_type) {
//                                 Some(DataType::Block) => {
//                                     let start = Instant::now();
//                                     let block: EthereumChainBlock = ethereum_decode(&mut data.payload).unwrap();
//                                     let ethereum_block = EthereumBlock::from(&block.block);
//                                     let result = conn.build_transaction().read_write().run::<(),Error,_>(|| {
//                                         let transactions = block.block.transactions.iter().map(|tran|{
//                                             let mut transaction = EthereumTransaction::from(tran);
//                                             transaction.timestamp = ethereum_block.timestamp;
//                                             transaction
//                                         }).collect::<Vec<EthereumTransaction>>();
//                                         match diesel::insert_into(ethereum_block::table)
//                                             .values(&ethereum_block)
//                                             .execute(&conn) {
//                                             Ok(_) => {}
//                                             Err(err) => log::error!("{:?}",&err)
//                                         }
//                                         let res = diesel::insert_into(ethereum_transaction::table)
//                                             .values(&transactions)
//                                             .execute(&conn);
//                                         match diesel::insert_into(network_state::table)
//                                             .values((network_state::chain.eq(CHAIN.clone()),
//                                                      network_state::network.eq(network.clone().unwrap_or(DEFAULT_NETWORK.to_string())),
//                                                      network_state::got_block.eq(ethereum_block.block_number.unwrap()))
//                                             )
//                                             .on_conflict((network_state::chain, network_state::network))
//                                             .do_update()
//                                             .set(network_state::got_block.eq(excluded(network_state::got_block)))
//                                             .execute(&conn) {
//                                                 Ok(_) => {}
//                                                 Err(err) => log::error!("{:?}",&err)
//                                             };
//                                         info!(
//                                             "Dump Ethereum BLOCK: {} with {} transactions in {:?}. Result {:?}",
//                                             &block.block.number.unwrap().as_u64(),
//                                             ethereum_block.transaction_number,
//                                             start.elapsed(),
//                                             &res
//                                         );
//                                         Ok(())
//                                     });
//                                     match result {
//                                         Err(err) => log::error!("{:?}", &err),
//                                         Ok(_) => {}
//                                     };
//                                 }
//                                 _ => {
//                                     warn!("Not support this type in Ethereum");
//                                 }
//                             };
//                         }
//                     }
//                     _ => {
//                         log::info!("Error while get message from reader stream {:?}. Recreate stream", &response);
//                         opt_stream = None;
//                     }
//                 }
//             }
//         }
//     }
// }
