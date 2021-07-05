use clap::App;
use sp_core::{sr25519, H256 as Hash};
use massbit_chain_substrate::data_type::{
    SubstrateBlock as Block, SubstrateHeader as Header };
use std::sync::mpsc::channel;
use substrate_api_client::{Api, rpc::json_req, utils::FromHexString};
use env_logger;
use serde_json;
use std::error::Error;
use crate::grpc_stream::stream_mod::{GenericDataProto, ChainType, DataType};

use tokio::sync::broadcast;

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Substrate;
const VERSION:&str = "1";



fn create_generic_block_from_header(api:&Api<sr25519::Pair>, header:Header) -> Result<GenericDataProto, Box<dyn Error>> {
    // Get block number
    let block_number = header.number;
    // Get Call rpc to block hash
    let hash = api.get_request(json_req::chain_get_block_hash(Some(block_number)).to_string())?;
    let hash = hash.unwrap();
    let block_hash = Hash::from_hex(hash.clone());

    // Call RPC to get block
    let block = api.get_block::<Block>(Some(block_hash.unwrap())).unwrap().unwrap();
    Ok(_create_generic_block(hash,&block))
}

fn _create_generic_block(   block_hash: String,
                            block:&Block) -> GenericDataProto
{
    // Remove exstrinsics because cannot deserialize them. Maybe because of `Extrinsic: MaybeSerialize`
    // Todo: Deserialize exstrinsic
    let mut block = (*block).clone();
    //println!("Block content: {:?}",block);
    block.extrinsics = Vec::new();
    let generic_block = GenericDataProto{
        chain_type: CHAIN_TYPE as i32,
        version: VERSION.to_string(),
        data_type: DataType::Block as i32,
        block_hash: block_hash,
        block_number: block.header.number as u64,
        payload: serde_json::to_vec(&block).unwrap()
    };
    // For decode:
    // let decode_block: Block = serde_json::from_slice(&generic_block.payload).unwrap();
    // println!("decode_block: {:?}",tmp);
    generic_block
}

pub async fn get_data(chan: broadcast::Sender<GenericDataProto>){

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
        loop{
            rx.recv().await.unwrap();
        }
    });

    loop{
        // Get new header
        let head: Header = recv
            .recv()
            .map(|header| serde_json::from_str(&header).unwrap())
            .unwrap();
        // Call rpc to create block from header
        let generic_block_proto = create_generic_block_from_header(&api, head).unwrap();
        println!("Got block number: {:?}, hash: {:?}", &generic_block_proto.block_number,&generic_block_proto.block_hash);

        println!("Sending generic data");
        chan.send(generic_block_proto).unwrap();
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
