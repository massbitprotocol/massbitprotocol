use clap::App;
use sp_core::{sr25519, H256 as Hash};
use massbit_chain_substrate::data_type::{SubstrateBlock as Block,
                                         SubstrateHeader as Header,
                                         SubstrateEventRecord as EventRecord,
                                         SubstrateUncheckedExtrinsic as Extrinsic,
                                        };
use std::sync::mpsc::channel;
use substrate_api_client::{Api, rpc::json_req, utils::FromHexString};
use env_logger;
use serde_json;
use std::error::Error;
use crate::grpc_stream::stream_mod::{GenericDataProto, ChainType, DataType};
use pallet_timestamp::Call as TimestampCall;
use pallet_balances::Call as BalancesCall;
use tokio::sync::broadcast;

#[cfg(feature = "std")]
use node_template_runtime::{Call, AccountId};
use codec::{Decode, Encode};
use substrate_api_client::Metadata;
use std::convert::TryFrom;
use node_template_runtime::Event;
use system;
use pallet_balances;

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Substrate;
const VERSION:&str = "1";

fn get_block_and_hash_from_header(api:&Api<sr25519::Pair>, header:Header) -> Result<(Block,String), Box<dyn Error>> {
    // Get block number
    let block_number = header.number;
    // Get Call rpc to block hash
    let hash = api.get_request(json_req::chain_get_block_hash(Some(block_number)).to_string())?;
    let hash = hash.unwrap();
    let block_hash = Hash::from_hex(hash.clone());

    // Call RPC to get block
    let block = api.get_block::<Block>(Some(block_hash.unwrap())).unwrap().unwrap();
    Ok((block, hash))
}

fn _create_generic_extrinsic(   block_hash: String,
                            block:&Block) -> GenericDataProto
{
    let block = (*block).clone();
    //println!("**Block content: {:#?}",&block);

    let mut extrinsics: Vec<Vec<u8>> = Vec::new();

    for extrinsic in block.extrinsics.clone(){
        extrinsics.push(extrinsic.encode());

    }
    let payload = extrinsics.encode();

    let generic_data = GenericDataProto{
        chain_type: CHAIN_TYPE as i32,
        version: VERSION.to_string(),
        data_type: DataType::Transaction as i32,
        block_hash,
        block_number: block.header.number as u64,
        payload,
    };
    // For decode:
    let encode_extrinsics: Vec<Vec<u8>> =  Decode::decode(&mut generic_data.payload.as_slice()).unwrap();
    for encode_extrinsic in encode_extrinsics{
        let decode_extrinsic: Extrinsic = Decode::decode(&mut encode_extrinsic.as_slice()).unwrap();
        println!("decode_extrinsic: {:?}", decode_extrinsic);
    }

    generic_data
}
fn _create_generic_block(   block_hash: String,
                            block:&Block) -> GenericDataProto
{
    let block = (*block).clone();

    let generic_data = GenericDataProto{
        chain_type: CHAIN_TYPE as i32,
        version: VERSION.to_string(),
        data_type: DataType::Block as i32,
        block_hash: block_hash,
        block_number: block.header.number as u64,
        payload: block.encode(),
    };
    generic_data
}

fn _create_generic_event(event: &system::EventRecord<Event, Hash>) -> GenericDataProto
{
    let generic_data = GenericDataProto{
        chain_type: CHAIN_TYPE as i32,
        version: VERSION.to_string(),
        data_type: DataType::Event as i32,
        block_hash: "unknown".to_string(),
        block_number: 0 as u64,
        payload: event.encode(),
    };
    generic_data
}

async fn get_event(chan: broadcast::Sender<GenericDataProto>) {

    let url = get_node_url_from_cli();
    let api = Api::<sr25519::Pair>::new(url).unwrap();

    println!("Subscribe to events");
    let (events_in, events_out) = channel();
    api.subscribe_events(events_in).unwrap();

    loop {
        let event_str = events_out.recv().unwrap();

        let _unhex = Vec::from_hex(event_str).unwrap();
        let mut _er_enc = _unhex.as_slice();
        let _events = Vec::<system::EventRecord<Event, Hash>>::decode(&mut _er_enc);

        match _events {
            Ok(evts) => {
                for evt in &evts {
                    let generic_data_proto = _create_generic_event(evt);
                    println!("Sending event as generic data: {:?}",generic_data_proto);
                    chan.send(generic_data_proto).unwrap();
                }
            }
            Err(_) => println!("couldn't decode event record list"),
        }
    }
}

pub async fn get_data(chan: broadcast::Sender<GenericDataProto>) {

    println!("start");
    env_logger::init();
    let url = get_node_url_from_cli();
    let api = Api::<sr25519::Pair>::new(url).unwrap();

    println!("Subscribing to finalized heads");
    let (send, recv) = channel();
    api.subscribe_finalized_heads(send).unwrap();

    let mut rx = chan.subscribe();
    // Todo: More clean solution for broadcast channel
    tokio::spawn(async move {
        loop {
            rx.recv().await.unwrap();
        }
    });

    let clone_chan = chan.clone();
    // Spam thread
    tokio::spawn(async move {
        get_event(clone_chan).await;
    });

    loop {
        // Get new header
        let head: Header = recv
            .recv()
            .map(|header| serde_json::from_str(&header).unwrap())
            .unwrap();
        // Call rpc to create block from header
        let (block, hash) = get_block_and_hash_from_header(&api, head).unwrap();
        let generic_block = _create_generic_block(hash.clone(), &block);
        // Send block
        println!("Got block number: {:?}, hash: {:?}", &generic_block.block_number, &generic_block.block_hash);
        //println!("Sending block as generic data {:?}", &generic_block);
        chan.send(generic_block).unwrap();

        // Send array of extrinsics
        let generic_extrinsics = _create_generic_extrinsic(hash, &block);
        println!("Sending extrinsics as generic data {:?}", &generic_extrinsics);
        chan.send(generic_extrinsics).unwrap();
    }
}

pub fn get_node_url_from_cli() -> String {
    let yml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yml).get_matches();

    let node_ip = matches.value_of("node-server").unwrap_or("ws://127.0.0.1");
    let node_port = matches.value_of("node-port").unwrap_or("9944");
    let url = format!("{}:{}", node_ip, node_port);
    println!("Interacting with node on {}\n", url);
    url
}
