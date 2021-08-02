use log::{debug, warn, error, info, Level};
use tokio::sync::broadcast;
use crate::{grpc_stream::stream_mod::{GenericDataProto, ChainType, DataType}, CONFIG};
use solana_client::{pubsub_client::PubsubClient, rpc_client::RpcClient};
use solana_transaction_status::UiTransactionEncoding;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
    thread
};
use std::error::Error;
use massbit_chain_solana::data_type::{  SolanaEncodedBlock as Block,
                                        get_list_log_messages_from_encoded_block,
                                        };
use std::sync::Mutex;


// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Solana;
const VERSION:&str = "1.6.16";
const BLOCK_AVAILABLE_MARGIN: u64 = 100;
const RPC_BLOCK_ENCODING: UiTransactionEncoding = UiTransactionEncoding::Base64;

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
    info!("Start get block Solana");
    let config = CONFIG.chains.get(&CHAIN_TYPE).unwrap();
    let json_rpc_url = config.url.clone();
    let websocket_url = config.ws.clone();
    info!("Init Solana client");
    {
        let (mut subscription_client, receiver) =
            PubsubClient::slot_subscribe(&websocket_url).unwrap();
        info!("Finished init Solana client");

        let main_latest_root = Arc::new(Mutex::new(0_u64));
        let receiver_latest_root = Arc::clone(&main_latest_root);
        //Listen for incomming message
        let thread_receiver = tokio::spawn(async move {
            let start = Instant::now();
            let mut startblock = 0_u64;
            loop {
                if let Ok(slot) = receiver.recv() {
                    let mut root = receiver_latest_root.lock().unwrap();
                    let mut rate = 0_f32;
                    if startblock == 0 {
                        startblock = slot.root;
                    } else {
                        rate = (slot.root - startblock) as f32/ start.elapsed().as_secs_f32();
                    }
                    if slot.root > *root + BLOCK_AVAILABLE_MARGIN {
                        info!("[RECEIVER] Received from websocket root block: {} at rate {} blocks/s", slot.root, rate);
                        *root = slot.root - BLOCK_AVAILABLE_MARGIN;
                    }
                }
            }
        });
        let exit = Arc::new(AtomicBool::new(false));
        let client = Arc::new(RpcClient::new(json_rpc_url.clone()));
        let mut last_root: u64 = 0;
        fix_one_thread_not_receive(&chan);
        loop {
            if exit.load(Ordering::Relaxed) {
                eprintln!("{}", "exit".to_string());
                subscription_client.shutdown().unwrap();
                break;
            }
            let latest_root = *main_latest_root.lock().unwrap();
            if latest_root == 0 || latest_root == last_root {
                continue;
            } else if last_root == 0 {
                last_root = latest_root;
            } else {
                //0 < last_root < latest_root
                info!("[MAIN] Latest stable block: {}, Pending block: {}", latest_root, latest_root - last_root);
                if let Ok(_block_height) = client.get_block_height() {
                    for block_height in last_root..latest_root {
                        let new_client = client.clone();
                        let chan_clone = chan.clone();
                        tokio::spawn(async move {
                            if let Ok(block) = get_block(new_client, block_height) {
                                let generic_data_proto = _create_generic_block(block.block.blockhash.clone(), block_height, &block);
                                info!("[CHILD] Sending generic data: {:?} of SOLANA  block: {}", &generic_data_proto.block_number, block_height);
                                chan_clone.send(generic_data_proto).unwrap();
                            }
                        });
                    }
                    last_root = latest_root;
                }
            }
        }
    }
}


pub async fn loop_get_block_old(chan: broadcast::Sender<GenericDataProto>) {
    info!("Start get block Solana");
    let config = CONFIG.chains.get(&CHAIN_TYPE).unwrap();
    let json_rpc_url = config.url.clone();
    let websocket_url = config.ws.clone();
    info!("Init Solana client");
    let (mut subscription_client, receiver) =
        PubsubClient::slot_subscribe(&websocket_url).unwrap();
    info!("Finished init Solana client");
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
                info!("Root: {:?}",new_info.root);
                match last_root {
                    Some(value_last_root) => {
                        if root == last_root.unwrap() {
                            continue;
                        }

                        for block_height in value_last_root..root{
                            let new_client = client.clone();
                            let chan_clone = chan.clone();
                            tokio::spawn(async move {
                                if let Ok(block) = get_block(new_client, block_height) {
                                    let generic_data_proto = _create_generic_block(block.block.blockhash.clone(), block_height, &block);
                                    info!("Sending SOLANA as generic data: {:?}", &generic_data_proto.block_number);
                                    //info!("Sending SOLANA as generic data");
                                    chan_clone.send(generic_data_proto).unwrap();
                                }
                            });
                        }
                        last_root = Some(root);
                    },
                    _ => last_root = Some(root),
                };
                info!("Got Block: {:?}", &last_root.unwrap());
            }
            Err(err) => {
                eprintln!("disconnected: {}", err);
                break;
            }
        }
    }
}


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

fn get_block(client: Arc<RpcClient>, block_height: u64) -> Result<Block,Box<dyn Error>>{

    info!("Starting get Block {}",block_height);
    let now = Instant::now();
    let block = client.get_block_with_encoding(block_height, RPC_BLOCK_ENCODING);
    let elapsed = now.elapsed();
    match block{
        Ok(block) => {
            let timestamp = (&block).block_time.unwrap();
            let list_log_messages = get_list_log_messages_from_encoded_block(&block);
            info!("Finished get Block: {:?}, time: {:?}, hash: {}", block_height, elapsed, &block.blockhash);
            let ext_block = Block {
                version: VERSION.to_string(),
                block,
                timestamp,
                list_log_messages,
            };
            Ok(ext_block)
        },
        _ => {
            //error!("Cannot get: {:?}", &block);
            Err(format!("Error cannot get block").into())
        },
    }

}