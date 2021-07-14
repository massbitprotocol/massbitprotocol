/// Solana chain-reader test code
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

fn solana_slot_subscribe(websocket_url: &String) {
    let (mut subscription_client, receiver) =
        PubsubClient::slot_subscribe(&websocket_url).unwrap();
    let spacer = "|";
    let exit = Arc::new(AtomicBool::new(false));
    let mut current: Option<SlotInfo> = None;
    let mut message = "".to_string();
    let mut last_root = std::u64::MAX;
    let mut last_root_update = Instant::now();
    let mut slots_per_second = std::f64::NAN;
    loop {
        if exit.load(Ordering::Relaxed) {
            eprintln!("{}",message.to_string());
            subscription_client.shutdown().unwrap();
            break;
        }

        match receiver.recv() {
            Ok(new_info) => {
                if last_root == std::u64::MAX {
                    last_root = new_info.root;
                    last_root_update = Instant::now();
                }
                if last_root_update.elapsed().as_secs() >= 5 {
                    let root = new_info.root;
                    slots_per_second =
                        (root - last_root) as f64 / last_root_update.elapsed().as_secs() as f64;
                    last_root_update = Instant::now();
                    last_root = root;
                }

                message = if slots_per_second.is_nan() {
                    format!("{:?}", new_info)
                } else {
                    format!(
                        "{:?} | root slot advancing at {:.2} slots/second",
                        new_info, slots_per_second
                    )
                };
                println!("{}", message.clone());

                if let Some(previous) = current {
                    let slot_delta: i64 = new_info.slot as i64 - previous.slot as i64;
                    let root_delta: i64 = new_info.root as i64 - previous.root as i64;

                    //
                    // if slot has advanced out of step with the root, we detect
                    // a mismatch and output the slot information
                    //
                    if slot_delta != root_delta {
                        let prev_root = format!(
                            "|<--- {} <- … <- {} <- {}   (prev)",
                            previous.root, previous.parent, previous.slot
                        );
                        println!("{:?}",&prev_root);

                        let new_root = format!(
                            "|  '- {} <- … <- {} <- {}   (next)",
                            new_info.root, new_info.parent, new_info.slot
                        );
                        println!("{}", prev_root);
                        println!("{}", new_root);
                        println!("{}", spacer);
                    }
                }
                current = Some(new_info);
            }
            Err(err) => {
                eprintln!("disconnected: {}", err);
                break;
            }
        }
    }
}

async fn get_block(client: Arc<RpcClient>, block_height: u64) -> Result<EncodedConfirmedBlock,Box<dyn Error>>{

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

async fn get_blocks(client: Arc<RpcClient>, start_block: u64, end_block: u64) {
    for block_height in start_block..end_block{
        let new_client = client.clone();
            tokio::spawn(async move {
            get_block(new_client,block_height).await;
        });
    }
}

async fn solana_finalized_block_subscribe(websocket_url: &String, json_rpc_url: &String) {
    let (mut subscription_client, receiver) =
        PubsubClient::slot_subscribe(&websocket_url).unwrap();
    let exit = Arc::new(AtomicBool::new(false));
    let client = Arc::new(RpcClient::new(json_rpc_url.clone()));

    let mut last_root: Option<u64> = None;

    loop {
        if exit.load(Ordering::Relaxed) {
            eprintln!("{}","exit".to_string());
            subscription_client.shutdown().unwrap();
            break;
        }

        match receiver.recv() {
            Ok(new_info) => {
                // Root is finalized block in Solana
                let root = new_info.root-100;
                println!("Root: {:?}",new_info.root);
                let block_height = client.get_block_height().unwrap();
                println!("Highest Block height: {:?}",&block_height);

                match last_root {
                    Some(value_last_root) => {
                        if root == last_root.unwrap() {
                            continue;
                        }
                        get_blocks(client.clone(),value_last_root, root).await;
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


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let json_rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    let websocket_url = "wss://api.mainnet-beta.solana.com".to_string();

    //solana_slot_subscribe(&websocket_url);
    solana_finalized_block_subscribe(&websocket_url, &json_rpc_url).await;


    let client = RpcClient::new(json_rpc_url);
    let block_height = client.get_block_height().unwrap();
    let block = client.get_block(block_height).unwrap();
    println!("{:?}",block_height);
    println!("Original block: {:?}",&block);
    let payload = serde_json::to_vec(&block).unwrap();
    let decode_block: EncodedConfirmedBlock = serde_json::from_slice(&payload).unwrap();
    println!("Decode: {:#?}", &decode_block);
    assert_eq!(block,decode_block);


    Ok(())
}
