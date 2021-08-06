
use web3::{
    futures::{future, StreamExt},
    types::{
        Address, Block, BlockId, BlockNumber as Web3BlockNumber, Bytes, CallRequest,
        Filter, FilterBuilder, Log, Transaction, TransactionReceipt, H256,
    }
};

use std::error::Error;

#[tokio::main]
async fn main() -> web3::Result<()> {
    let _ = env_logger::try_init();
    let transport = web3::transports::WebSocket::new("wss://main-light.eth.linkpool.io/ws").await?;
    let web3 = web3::Web3::new(transport);

    let mut sub = web3.eth_subscribe().subscribe_new_heads().await?;

    println!("Got subscription id: {:?}", sub.id());

    loop {
        let mut headers = Vec::new();
        (&mut sub)
            .take(1)
            .for_each(|x| {
                println!("Got: {:?}", x);
                headers.push(x);
                future::ready(())
            })
            .await;

        for header in headers {
            let block = web3.eth()
                .block_with_txs(BlockId::Hash(header.unwrap().hash.unwrap())).await;
            println!("Block: {:?}",block);
        }
    }

}