pub mod handler;
pub mod metrics;
pub mod model;

use crate::postgres_adapter::PostgresAdapter;
use crate::schema::network_state;
use crate::solana::handler::create_solana_handler_manager;
use crate::{
    establish_connection, get_block_number, try_create_stream, GET_BLOCK_TIMEOUT_SEC,
    GET_STREAM_TIMEOUT_SEC,
};
use lazy_static::lazy_static;
use massbit::firehose::stream::{stream_client::StreamClient, BlockResponse, ChainType};
use massbit_chain_solana::data_type::{
    convert_solana_encoded_block_to_solana_block, decode as solana_decode, SolanaEncodedBlock,
};
use massbit_common::prelude::diesel::pg::upsert::excluded;
use massbit_common::prelude::diesel::{ExpressionMethods, RunQueryDsl};
use massbit_common::prelude::tokio::time::{sleep, timeout, Duration};
use massbit_common::NetworkType;
use std::sync::Arc;
use std::time::Instant;
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status, Streaming,
};
use tower::timeout::Timeout;

lazy_static! {
    pub static ref CHAIN: String = String::from("solana");
}
const START_SOLANA_BLOCK: u64 = 80_000_000_u64;
const DEFAULT_NETWORK: &str = "mainnet";

pub async fn process_solana_stream(
    client: &mut StreamClient<Timeout<Channel>>,
    storage_adapter: Arc<PostgresAdapter>,
    network_name: Option<NetworkType>,
    block: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let network = match network_name {
        None => Some(String::from(DEFAULT_NETWORK)),
        n @ Some(..) => n,
    };
    let handler_manager = Arc::new(create_solana_handler_manager(&network, storage_adapter));
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
                START_SOLANA_BLOCK
            }
        }
        Some(state) => state.got_block as u64 + 1,
    };
    let mut opt_stream: Option<Streaming<BlockResponse>> = None;
    loop {
        match opt_stream {
            None => {
                opt_stream =
                    try_create_stream(client, ChainType::Solana, start_block.clone(), &network)
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
                            let start = Instant::now();
                            let encoded_block: SolanaEncodedBlock =
                                solana_decode(&mut data.payload).unwrap();
                            // Decode
                            let block = convert_solana_encoded_block_to_solana_block(encoded_block);
                            let block_number = block.block.block_height.unwrap() as i64;
                            let transaction_counter = block.block.transactions.len();
                            log::info!(
                                "Decode block {} with {} transaction in {:?}",
                                block_number,
                                block.block.transactions.len(),
                                start.elapsed()
                            );
                            let start = Instant::now();
                            let handler = handler_manager.clone();
                            tokio::spawn(async move {
                                match handler.handle_block(Arc::new(block)) {
                                    Ok(_) => {}
                                    Err(err) => log::error!("{:?}", &err),
                                };
                            });
                            match diesel::insert_into(network_state::table)
                                .values((
                                    network_state::chain.eq(CHAIN.clone()),
                                    network_state::network
                                        .eq(network.clone().unwrap_or(DEFAULT_NETWORK.to_string())),
                                    network_state::got_block.eq(block_number),
                                ))
                                .on_conflict((network_state::chain, network_state::network))
                                .do_update()
                                .set(
                                    network_state::got_block.eq(excluded(network_state::got_block)),
                                )
                                .execute(&conn)
                            {
                                Ok(_) => {}
                                Err(err) => log::error!("{:?}", &err),
                            };
                            log::info!(
                                "Block height {} with {} transactions is processed in {:?}",
                                block_number,
                                transaction_counter,
                                start.elapsed()
                            );
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
}
