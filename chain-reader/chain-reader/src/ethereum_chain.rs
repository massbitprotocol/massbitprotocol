use log::{debug, warn, error, info, Level};
use tokio::sync::broadcast;
use crate::{grpc_stream::stream_mod::{GenericDataProto, ChainType, DataType}, CONFIG};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
    thread
};
use std::error::Error;
use web3;
use web3::{
    futures::{future, StreamExt},
    types::{
        Address, Block as EthBlock, BlockId, BlockNumber as Web3BlockNumber, Bytes, CallRequest,
        Filter, FilterBuilder, Log, Transaction, TransactionReceipt, H256,
    }
};
use massbit_chain_ethereum::data_type::{  EthereumBlock as Block, LightEthereumBlock};



// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Ethereum;

fn fix_one_thread_not_receive(chan: &broadcast::Sender<GenericDataProto>){
    // Todo: More clean solution for broadcast channel
    let mut rx = chan.subscribe();
    tokio::spawn(async move {
        loop {
            let _ = rx.recv().await;
        }
    });
}



pub async fn loop_get_block(chan: broadcast::Sender<GenericDataProto>) {
    info!("Start get block {:?}",CHAIN_TYPE);
    let config = CONFIG.chains.get(&CHAIN_TYPE).unwrap();
    let json_rpc_url = config.url.clone();
    let websocket_url = config.ws.clone();

    info!("Init Ethereum adapter");
    let exit = Arc::new(AtomicBool::new(false));

    let transport = web3::transports::WebSocket::new(websocket_url.as_str()).await.unwrap();
    let web3 = web3::Web3::new(transport);

    // Get version
    let version = web3.net().version().await.unwrap();

    let mut sub = web3.eth_subscribe().subscribe_new_heads().await.unwrap();
    println!("Got subscription id: {:?}", sub.id());

    let mut last_indexed_slot: Option<u64> = None;
    fix_one_thread_not_receive(&chan);
    loop {
        if exit.load(Ordering::Relaxed) {
            eprintln!("{}","exit".to_string());
            sub.unsubscribe().await;
            break;
        }
        // Get wait for header from chain
        let mut headers = Vec::new();
        (&mut sub)
            .take(1)
            .for_each(|header| {
                info!("Got header with hash: {:?}", header.clone().unwrap().hash);
                headers.push(header.unwrap());
                future::ready(())
            })
            .await;


        for header in headers {
            let clone_web3 = web3.clone();
            let clone_version = version.clone();
            let chan_clone = chan.clone();
            tokio::spawn(async move {
                let block_hash = header.hash.unwrap();
                let block_number = header.number.unwrap().as_u64();
                // Get block
                let block = clone_web3
                    .eth()
                    .block_with_txs(BlockId::Hash(block_hash)).await;

                if let Ok(Some(block)) = block {
                    //println!("Got ETHEREUM Block {:?}",block);
                    // Convert to generic

                    let eth_block = Block{
                        version: clone_version.clone(),
                        timestamp: block.timestamp.as_u64(),
                        block,
                        // Todo: Add receipts. Now hardcode empty.
                        receipts: vec![]
                    };

                    let generic_data_proto = _create_generic_block(
                        block_hash.to_string(),
                        block_number,
                        &eth_block,clone_version);
                    info!("Sending ETHEREUM as generic data: {:?}", &generic_data_proto.block_number);
                    chan_clone.send(generic_data_proto).unwrap();
                }
            });
        }

    }
}


fn _create_generic_block(   block_hash: String,
                            block_number: u64,
                            block: &Block,
                            version: String,
                        ) -> GenericDataProto
{

    let generic_data = GenericDataProto{
        chain_type: CHAIN_TYPE as i32,
        version,
        data_type: DataType::Block as i32,
        block_hash,
        block_number,
        payload: serde_json::to_vec(block).unwrap(),
    };
    generic_data
}

