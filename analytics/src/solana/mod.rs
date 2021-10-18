pub mod handler;
pub mod metrics;
pub mod model;
pub mod processor;
pub mod reader;

use crate::postgres_adapter::{create_r2d2_connection_pool, PostgresAdapter};
use crate::schema::network_state;
use crate::solana::handler::create_solana_handler_manager;
use crate::{
    establish_connection, get_block_number, try_create_stream, DEFAULT_DATABASE_URL,
    GET_BLOCK_TIMEOUT_SEC, GET_STREAM_TIMEOUT_SEC, MAX_POOL_SIZE,
};
use core::ops::Deref;
use diesel::PgConnection;
use lazy_static::lazy_static;
use massbit::firehose::bstream::{stream_client::StreamClient, BlockResponse, ChainType};
use massbit_chain_solana::data_type::{
    convert_solana_encoded_block_to_solana_block, decode as solana_decode, SolanaBlock,
    SolanaEncodedBlock,
};
use massbit_common::prelude::diesel::pg::upsert::excluded;
use massbit_common::prelude::diesel::r2d2::{ConnectionManager, PooledConnection};
use massbit_common::prelude::diesel::{ExpressionMethods, RunQueryDsl};
use massbit_common::prelude::r2d2::Error;
use massbit_common::prelude::tokio::time::{sleep, timeout, Duration};
use massbit_common::NetworkType;
use std::env;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc::Receiver;

pub use processor::process_solana_channel;
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status, Streaming,
};
use tower::timeout::Timeout;
lazy_static! {
    pub static ref CHAIN: String = String::from("solana");
    pub static ref SOLANA_WS: String = env::var("SOLANA_WS").unwrap_or(String::from("ws://api.mainnet-beta.solana.com"));
    //static ref SOLANA_URL: String = env::var("SOLANA_URL").unwrap_or(String::from("https://solana-api.projectserum.com"));
    pub static ref SOLANA_URL: String = env::var("SOLANA_URL").unwrap_or(String::from("http://194.163.156.242:8899"));
}
//const START_SOLANA_BLOCK: i64 = 80_000_000_i64;
const DEFAULT_NETWORK: &str = "mainnet";

// pub async fn process_solana_stream(
//     client: &mut StreamClient<Timeout<Channel>>,
//     storage_adapter: Arc<PostgresAdapter>,
//     network_name: Option<NetworkType>,
//     block: Option<u64>,
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
//     let network = network_name.or(Some((String::from(DEFAULT_NETWORK))));
//     let handler_manager = Arc::new(create_solana_handler_manager(
//         &network,
//         storage_adapter.clone(),
//     ));
//     //Todo: remove this simple connection
//     //let conn = establish_connection();
//     let current_state = storage_adapter.get_connection().ok().and_then(|conn| {
//         get_block_number(
//             conn.deref(),
//             CHAIN.clone(),
//             network.clone().unwrap_or(String::from(DEFAULT_NETWORK)),
//         )
//     });
//     // let current_state = get_block_number(
//     //     &conn,
//     //     CHAIN.clone(),
//     //     network.clone().unwrap_or(String::from(DEFAULT_NETWORK)),
//     // );
//     let start_block = current_state
//         .and_then(|state| Some(state.got_block as u64 + 1))
//         .or(block);
//     let mut opt_stream: Option<Streaming<BlockResponse>> = None;
//     let mut last_block = Arc::new(Mutex::new(0_u64));
//     loop {
//         match opt_stream {
//             None => {
//                 opt_stream =
//                     try_create_stream(client, ChainType::Solana, start_block, &network).await;
//                 if opt_stream.is_none() {
//                     //Sleep for a while and reconnect
//                     sleep(Duration::from_secs(GET_STREAM_TIMEOUT_SEC)).await;
//                 }
//             }
//             Some(ref mut stream) => {
//                 let response =
//                     timeout(Duration::from_secs(GET_BLOCK_TIMEOUT_SEC), stream.message()).await;
//                 match response {
//                     Ok(Ok(res)) => {
//                         if let Some(mut data) = res {
//                             let block_slot = data.block_number;
//                             let handler = handler_manager.clone();
//                             let storage_adapter = storage_adapter.clone();
//                             let network_name = network.clone();
//                             let last_block = Arc::clone(&last_block);
//                             tokio::spawn(async move {
//                                 let start = Instant::now();
//                                 let block: SolanaBlock = solana_decode(&mut data.payload).unwrap();
//                                 // Decode
//                                 //let block = convert_solana_encoded_block_to_solana_block(encoded_block);
//                                 let transaction_counter = block.block.transactions.len();
//                                 log::info!(
//                                     "Decode block {} with {} transaction in {:?}",
//                                     block_slot,
//                                     block.block.transactions.len(),
//                                     start.elapsed()
//                                 );
//                                 let start = Instant::now();
//                                 match handler.handle_block(block_slot, Arc::new(block)) {
//                                     Ok(_) => {}
//                                     Err(err) => log::error!("{:?}", &err),
//                                 };
//                                 let mut last_block = last_block.lock().unwrap();
//                                 if *last_block < block_slot {
//                                     *last_block = block_slot;
//                                     if let Ok(conn) = storage_adapter.get_connection() {
//                                         match diesel::insert_into(network_state::table)
//                                             .values((
//                                                 network_state::chain.eq(CHAIN.clone()),
//                                                 network_state::network
//                                                     .eq(network_name.unwrap_or_default()),
//                                                 network_state::got_block.eq(block_slot as i64),
//                                             ))
//                                             .on_conflict((
//                                                 network_state::chain,
//                                                 network_state::network,
//                                             ))
//                                             .do_update()
//                                             .set(
//                                                 network_state::got_block
//                                                     .eq(excluded(network_state::got_block)),
//                                             )
//                                             .execute(conn.deref())
//                                         {
//                                             Ok(_) => {}
//                                             Err(err) => log::error!("{:?}", &err),
//                                         };
//                                     };
//                                 }
//
//                                 log::info!(
//                                     "Block slot {} with {} transactions is processed in {:?}",
//                                     block_slot,
//                                     transaction_counter,
//                                     start.elapsed()
//                                 );
//                             });
//                         }
//                     }
//                     _ => {
//                         log::info!(
//                             "Error while get message from reader stream {:?}. Recreate stream",
//                             &response
//                         );
//                         opt_stream = None;
//                     }
//                 }
//             }
//         };
//     }
// }
