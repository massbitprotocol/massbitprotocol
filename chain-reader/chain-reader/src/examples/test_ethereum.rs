use anyhow::Error;
use chain_reader::Transport;
use futures::prelude::*;
use futures03::{self, compat::Future01CompatExt};
use std::time::Instant;
use thiserror::Error;
use web3::api::Web3;
use web3::types::{
    Address, Block, BlockId, BlockNumber as Web3BlockNumber, Bytes, CallRequest, Filter,
    FilterBuilder, Log, Transaction, TransactionReceipt, H160, H256,
};

use chain_reader::ethereum_chain::{get_logs, get_receipts};
use std::str::FromStr;

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

    // let (_eloop, ws) =
    //     web3::transports::WebSocket::new("wss://bsc-ws-node.nariox.org:443").unwrap();
    // let web3 = web3::Web3::new(ws.clone());
    //
    // let mut sub = web3.eth_subscribe().subscribe_new_heads().wait().unwrap();
    //
    // println!("Got subscription id: {:?}", sub.id());
    //
    // (&mut sub)
    //     .take(5)
    //     .for_each(|x| {
    //         println!("Got: {:?}", x);
    //         Ok(())
    //     })
    //     .wait()
    //     .unwrap();

    let is_ws = false;

    // let url_ws = "wss://main-light.eth.linkpool.io/ws";
    //let url_ws = "wss://bsc-ws-node.nariox.org:443";
    let url_ws = "wss://rpc-mainnet.matic.network";

    // let url_http =  "https://main-light.eth.linkpool.io";
    //let url_http = "https://bsc-dataseed.binance.org";
    let url_http = "https://rpc-mainnet.matic.network";
    //let url_http = "https://matic-mainnet.chainstacklabs.com";
    //let url_http = "https://rpc-mainnet.maticvigil.com";

    let (transport_event_loop, transport) = match is_ws {
        false => Transport::new_rpc(&url_http, Default::default()),
        true => Transport::new_ws(&url_ws),
    };
    std::mem::forget(transport_event_loop);
    let web3 = Web3::new(transport);

    let from = Web3BlockNumber::Latest;
    let to = Web3BlockNumber::Latest;
    println!("Got adapter");
    // Address of QuickSwap
    let address: H160 = H160::from_str("5757371414417b8C6CAad45bAeF941aBc7d3Ab32").unwrap();
    let mut addresses = Vec::new();
    addresses.push(address);
    // Create a log filter
    let log_filter: Filter = FilterBuilder::default()
        .from_block(from.into())
        .to_block(to.into())
        .address(addresses)
        //.topics(Some(filter.event_signatures.clone()), None, None, None)
        .build();

    loop {
        let logs = get_logs(&web3, from, to);
        println!("Logs: {:?}", logs);
    }

    // Test speed
    for _ in 0..1 {
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
