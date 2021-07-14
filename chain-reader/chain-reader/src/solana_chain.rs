// use chain_reader::CONFIG;
use tokio::sync::broadcast;
use crate::{grpc_stream::stream_mod::{GenericDataProto, ChainType, DataType}, CONFIG};

use solana_client::{pubsub_client::PubsubClient, rpc_client::RpcClient, rpc_response::SlotInfo};
use solana_transaction_status::{UiConfirmedBlock, EncodedConfirmedBlock};
use codec::{Decode, Encode};
//use serde::Serialize;
use serde_json::{Serializer, Deserializer};
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Instant}
};
use std::error::Error;
use massbit_chain_solana::data_type::{SolanaBlock as Block};

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Solana;
const VERSION:&str = "1.6.16";
const BLOCK_AVAILABLE_MARGIN: u64 = 100;

fn fix_one_thread_not_receive(chan: &broadcast::Sender<GenericDataProto>){
    // Todo: More clean solution for broadcast channel
    let mut rx = chan.subscribe();
    tokio::spawn(async move {
        loop {
            rx.recv().await.unwrap();
        }
    });
}

pub async fn loop_get_block(chan: broadcast::Sender<GenericDataProto>) {
    println!("start");
    let config = CONFIG.chains.get(&CHAIN_TYPE).unwrap();
    let json_rpc_url = config.url.clone();
    let websocket_url = config.ws.clone();

    let (mut subscription_client, receiver) =
        PubsubClient::slot_subscribe(&websocket_url).unwrap();
    let exit = Arc::new(AtomicBool::new(false));
    let client = Arc::new(RpcClient::new(json_rpc_url.clone()));

    let mut last_root: Option<u64> = None;
    fix_one_thread_not_receive(&chan);
    loop {
        if exit.load(Ordering::Relaxed) {
            eprintln!("{}","exit".to_string());
            subscription_client.shutdown().unwrap();
            break;
        }

        match receiver.recv() {
            Ok(new_info) => {
                // Root is finalized block in Solana
                let root = new_info.root-BLOCK_AVAILABLE_MARGIN;
                println!("Root: {:?}",new_info.root);
                let block_height = client.get_block_height().unwrap();
                println!("Highest Block height: {:?}",&block_height);

                match last_root {
                    Some(value_last_root) => {
                        if root == last_root.unwrap() {
                            continue;
                        }

                        //get_blocks(client.clone(), &chan, value_last_root, root);
                        for block_height in value_last_root..root{
                            let new_client = client.clone();
                            //tokio::spawn(async move {
                            let block = get_block(new_client,block_height);
                            match block {
                                Ok(block) => {
                                    let generic_data_proto = _create_generic_block(block.blockhash.clone(),block_height, &block);
                                    println!("Sending SOLANA as generic data: {:?}", &generic_data_proto.block_number);
                                    //println!("Sending SOLANA as generic data");
                                    chan.send(generic_data_proto).unwrap();
                                },
                                // Cannot get the block, pass
                                Err(_) => continue,
                            }

                            //});
                        }
                        last_root = Some(root);
                    },
                    _ => last_root = Some(root),
                };
                println!("Got Block: {:?}", &last_root.unwrap());
            }
            Err(err) => {
                eprintln!("disconnected: {}", err);
                break;
            }
        }
    }
}

// async fn solana_finalized_block_subscribe(websocket_url: &String, json_rpc_url: &String) {
//
// }

fn _create_generic_block(   block_hash: String,
                            block_number: u64,
                            block:&Block) -> GenericDataProto
{

    let generic_data = GenericDataProto{
        chain_type: CHAIN_TYPE as i32,
        version: VERSION.to_string(),
        data_type: DataType::Block as i32,
        block_hash,
        block_number,
        payload: serde_json::to_vec(block).unwrap(),
    };
    generic_data
}

fn get_blocks(client: Arc<RpcClient>, chan: &broadcast::Sender<GenericDataProto> , start_block: u64, end_block: u64) {
    for block_height in start_block..end_block{
        let new_client = client.clone();
        //tokio::spawn(async move {
        let block = get_block(new_client,block_height);
        match block {
            Ok(block) => {
                let generic_data_proto = _create_generic_block(block.blockhash.clone(),block_height, &block);
                //println!("Sending SOLANA as generic data: {:?}", &generic_data_proto.block_number);
                println!("Sending SOLANA as generic data");
                chan.send(generic_data_proto).unwrap();
            },
            // Cannot get the block, pass
            Err(_) => continue,
        }

        //});
    }
}

fn get_block(client: Arc<RpcClient>, block_height: u64) -> Result<EncodedConfirmedBlock,Box<dyn Error>>{

    println!("Starting get Block {}",block_height);
    let now = Instant::now();
    let block = client.get_block(block_height);
    let elapsed = now.elapsed();
    match block{
        Ok(block) => {
            println!("Finished get Block: {:?}, time: {:?}, hash: {}", block_height, elapsed, &block.blockhash);
            Ok(block)
        },
        _ => {
            println!("Cannot get: {:?}", &block);
            Err(format!("Error cannot get block").into())
        },
    }

}