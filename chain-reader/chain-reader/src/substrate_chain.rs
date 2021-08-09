use crate::grpc_stream::stream_mod::{ChainType, DataType, GenericDataProto};
use clap::App;
use env_logger;
use massbit_chain_substrate::data_type::{
    SubstrateBlock as Block, SubstrateEventRecord as EventRecord, SubstrateHeader as Header,
};
use serde_json;
use sp_core::{sr25519, H256 as Hash};
use std::error::Error;
use std::sync::mpsc::channel;
use substrate_api_client::{rpc::json_req, utils::FromHexString, Api};
use tokio::sync::broadcast;

#[cfg(feature = "std")]
use codec::{Decode, Encode};
use log::{debug, error, info, warn, Level};
use node_template_runtime::Block as OrgBlock;
use node_template_runtime::Event;
use std::env;
use system;

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Substrate;
const VERSION: &str = "1";

fn get_block_and_hash_from_header(
    api: &Api<sr25519::Pair>,
    header: Header,
) -> Result<(Block, String), Box<dyn Error>> {
    // Get block number
    let block_number = header.number;
    // Get Call rpc to block hash
    let hash = api.get_request(json_req::chain_get_block_hash(Some(block_number)).to_string())?;
    let hash = hash.unwrap();
    let block_hash = Hash::from_hex(hash.clone());

    // Call RPC to get block
    let block = api
        .get_block::<OrgBlock>(Some(block_hash.unwrap()))
        .unwrap()
        .unwrap();
    let ext_block = Block {
        version: VERSION.to_string(),
        // Todo: get correct timestamp from the Set_Timestamp extrinsic
        // https://github.com/paritytech/substrate/issues/2811
        timestamp: 0,
        block,
        // Todo: get events of the block and add here
        events: Vec::new(),
    };
    Ok((ext_block, hash))
}

fn _create_generic_block(block_hash: String, block: &Block) -> GenericDataProto {
    let block = (*block).clone();

    let generic_data = GenericDataProto {
        chain_type: CHAIN_TYPE as i32,
        version: VERSION.to_string(),
        data_type: DataType::Block as i32,
        block_hash: block_hash,
        block_number: block.block.header.number as u64,
        payload: block.encode(),
    };
    generic_data
}

fn _create_generic_event(event: &EventRecord) -> GenericDataProto {
    let generic_data = GenericDataProto {
        chain_type: CHAIN_TYPE as i32,
        version: VERSION.to_string(),
        data_type: DataType::Event as i32,
        block_hash: "unknown".to_string(),
        block_number: 0 as u64,
        payload: event.encode(),
    };
    generic_data
}

pub async fn loop_get_event(chan: broadcast::Sender<GenericDataProto>) {
    let url = get_node_url_from_cli();
    let api = Api::<sr25519::Pair>::new(url).unwrap();

    info!("Subscribe to events");
    let (events_in, events_out) = channel();
    api.subscribe_events(events_in).unwrap();

    fix_one_thread_not_receive(&chan);
    loop {
        let event_str = events_out.recv().unwrap();

        let _unhex = Vec::from_hex(event_str).unwrap();
        let mut _er_enc = _unhex.as_slice();
        let _events = Vec::<system::EventRecord<Event, Hash>>::decode(&mut _er_enc);

        match _events {
            Ok(evts) => {
                debug!("{:?}", evts);
                for evt in &evts {
                    let ext_event = EventRecord {
                        // Todo: Need find the block number and add here
                        // block_number: 0,
                        // Todo: Need find extrinsic and add here
                        // extrinsic: None,
                        // Todo: Need find the block add add here
                        // block: Box<ExtBlock>,
                        event: evt.clone(),
                        // Todo: Need find the success add add here
                    };
                    let generic_data_proto = _create_generic_event(&ext_event);
                    debug!(
                        "Sending SUBSTRATE event as generic data: {:?}",
                        generic_data_proto
                    );
                    chan.send(generic_data_proto).unwrap();
                }
            }
            Err(_) => error!("couldn't decode event record list"),
        }
    }
}

fn fix_one_thread_not_receive(chan: &broadcast::Sender<GenericDataProto>) {
    // Todo: More clean solution for broadcast channel
    let mut rx = chan.subscribe();
    tokio::spawn(async move {
        loop {
            rx.recv().await;
        }
    });
}

pub async fn loop_get_block_and_extrinsic(chan: broadcast::Sender<GenericDataProto>) {
    info!("Start get block and extrinsic Substrate");
    let url = get_node_url_from_cli();
    let api = Api::<sr25519::Pair>::new(url).unwrap();

    info!("Subscribing to finalized heads");
    let (send, recv) = channel();
    api.subscribe_finalized_heads(send).unwrap();

    fix_one_thread_not_receive(&chan);

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
        info!(
            "Got block number: {:?}, hash: {:?}",
            &generic_block.block_number, &generic_block.block_hash
        );
        chan.send(generic_block).unwrap();
    }
}

pub fn get_node_url_from_cli() -> String {
    let yml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yml).get_matches();

    let node_server = match env::var("NODE_SERVER") {
        Ok(connection) => connection, // Configuration from docker-compose environment
        Err(_) => String::from("ws://127.0.0.1"),
    };
    let node_ip = matches.value_of("node-server").unwrap_or(&node_server);
    let node_port = matches.value_of("node-port").unwrap_or("9944");
    let url = format!("{}:{}", node_ip, node_port);
    info!("Interacting with node on {}\n", url);
    url
}
