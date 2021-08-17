use anyhow::Error;
use chain_reader::Transport;
use futures::prelude::*;
use futures03::{self, compat::Future01CompatExt};
use std::time::Instant;
use thiserror::Error;
use web3::api::Web3;
use web3::types::{
    Address, Block, BlockId, BlockNumber as Web3BlockNumber, Bytes, CallRequest, Filter,
    FilterBuilder, Log, Transaction, TransactionReceipt, H256,
};

use chain_reader::ethereum_chain::get_receipts;

#[derive(Error, Debug)]
pub enum MyIngestorError {
    /// The Ethereum node does not know about this block for some reason, probably because it
    /// disappeared in a chain reorg.
    #[error("Block data unavailable, block was likely uncled (block hash = {0:?})")]
    BlockUnavailable(H256),

    /// An unexpected error occurred.
    #[error("Ingestor error: {0}")]
    Unknown(Error),
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let (_eloop, ws) =
        web3::transports::WebSocket::new("wss://bsc-ws-node.nariox.org:443").unwrap();
    let web3 = web3::Web3::new(ws.clone());

    let mut sub = web3.eth_subscribe().subscribe_new_heads().wait().unwrap();

    println!("Got subscription id: {:?}", sub.id());

    (&mut sub)
        .take(5)
        .for_each(|x| {
            println!("Got: {:?}", x);
            Ok(())
        })
        .wait()
        .unwrap();

    let is_ws = true;

    // let url_ws = "wss://main-light.eth.linkpool.io/ws";
    let url_ws = "wss://bsc-ws-node.nariox.org:443";
    // let url_http =  "https://main-light.eth.linkpool.io";
    let url_http = "https://bsc-dataseed.binance.org";

    let (transport_event_loop, transport) = match is_ws {
        false => Transport::new_rpc(&url_http, Default::default()),
        true => Transport::new_ws(&url_ws),
    };
    std::mem::forget(transport_event_loop);
    let web3 = Web3::new(transport);

    println!("Got adapter");

    // Test speed
    for _ in 0..10 {
        let block = web3
            .eth()
            .block_with_txs(Web3BlockNumber::Latest.into())
            .compat()
            .await
            .unwrap()
            .unwrap();
        println!("Got block: {:?}", block.hash.unwrap());

        let now = Instant::now();

        let receipts = get_receipts(&block, &web3).await;
        // println!("Receipts: {:#?}", receipts);
        println!("Number of receipts: {:#?}", receipts.len());
        let elapsed = now.elapsed();
        println!("Elapsed: {:.2?}", elapsed);
    }
}
