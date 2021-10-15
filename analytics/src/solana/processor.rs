use super::CHAIN;
use crate::get_block_number;
use crate::postgres_adapter::PostgresAdapter;
use crate::schema::network_state;
use crate::solana::handler::create_solana_handler_manager;
use crate::solana::model::EncodedConfirmedBlockWithSlot;
use core::ops::Deref;
use massbit::firehose::bstream::BlockResponse;
use massbit_chain_solana::data_type::{decode_encoded_block, SolanaBlock};
use massbit_common::prelude::diesel::pg::upsert::excluded;
use massbit_common::prelude::diesel::{ExpressionMethods, RunQueryDsl};
use massbit_common::NetworkType;
use solana_transaction_status::EncodedConfirmedBlock;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc::Receiver;
use tonic::Status;

const DEFAULT_NETWORK: &str = "mainnet";

pub async fn process_solana_channel(
    rx: &mut Receiver<EncodedConfirmedBlockWithSlot>,
    storage_adapter: Arc<PostgresAdapter>,
    network_name: &NetworkType,
    block: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let network = Some(network_name.clone());
    let handler_manager = Arc::new(create_solana_handler_manager(
        &network,
        storage_adapter.clone(),
    ));
    let current_state = storage_adapter.get_connection().ok().and_then(|conn| {
        get_block_number(
            conn.deref(),
            super::CHAIN.clone(),
            network.clone().unwrap_or(String::from(DEFAULT_NETWORK)),
        )
    });
    let start_block = current_state
        .and_then(|state| Some(state.got_block as u64 + 1))
        .or(block);
    let mut last_block = Arc::new(Mutex::new(0_u64));
    while let Some(mut data) = rx.recv().await {
        let block_slot = data.block_slot;
        let handler = handler_manager.clone();
        let storage_adapter = storage_adapter.clone();
        let network_name = network.clone();
        let last_block = Arc::clone(&last_block);
        tokio::spawn(async move {
            let start = Instant::now();
            //let block: SolanaBlock = solana_decode(&mut data.payload).unwrap();
            // Decode
            //let block = convert_solana_encoded_block_to_solana_block(encoded_block);
            //let block = decode_encoded_block(data.block);
            let block = data.block;
            let transaction_counter = block.transactions.len();
            log::info!(
                "Decode block {} with {} transaction in {:?}",
                block_slot,
                transaction_counter,
                start.elapsed()
            );
            let start = Instant::now();
            match handler.handle_confirmed_block(block_slot, Arc::new(data.block)) {
                Ok(_) => {}
                Err(err) => log::error!("{:?}", &err),
            };
            let mut last_block = last_block.lock().unwrap();
            if *last_block < block_slot {
                *last_block = block_slot;
                if let Ok(conn) = storage_adapter.get_connection() {
                    match diesel::insert_into(network_state::table)
                        .values((
                            network_state::chain.eq(CHAIN.clone()),
                            network_state::network.eq(network_name.unwrap_or_default()),
                            network_state::got_block.eq(block_slot as i64),
                        ))
                        .on_conflict((network_state::chain, network_state::network))
                        .do_update()
                        .set(network_state::got_block.eq(excluded(network_state::got_block)))
                        .execute(conn.deref())
                    {
                        Ok(_) => {}
                        Err(err) => log::error!("{:?}", &err),
                    };
                };
            }

            log::info!(
                "Block slot {} with {} transactions is processed in {:?}",
                block_slot,
                transaction_counter,
                start.elapsed()
            );
        });
    }
    Ok(())
}
