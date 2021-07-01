use clap::App;

use sp_core::{sr25519, H256 as Hash};

use massbit_chain_substrate::data_type::{
    SubstrateBlock as Block, SubstrateHeader as Header };
use std::sync::mpsc::channel;
use substrate_api_client::Api;
use substrate_api_client::rpc::json_req;
use substrate_api_client::utils::FromHexString;
use env_logger;
use serde_json;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::stream_mod::GenericDataProto;
use prost;
//use multiqueue;
use broadcaster::BroadcastChannel;
use futures_util::StreamExt;

const CHAIN_TYPE: ChainType = ChainType::Substrate;
const VERSION:&str = "1";

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone, prost::Enumeration)]
pub enum ChainType{
    Substrate = 0,
    Ethereum = 1,
    Solana = 2,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone, prost::Enumeration)]
pub enum DataType{
    Block = 0,
    Event = 1,
    Transaction = 2,
}


#[derive(Debug, Clone)]
pub struct GenericData{
    chain_type: ChainType,
    version: String,
    data_type: DataType,
    block_hash: String,
    block_number: u64,
    payload: Vec<u8>
}

fn create_generic_block_from_header(api:&Api<sr25519::Pair>, header:Header) -> Result<GenericData, Box<dyn Error>> {
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

fn create_generic_data_proto_from_generic_data(generic_data: GenericData) -> GenericDataProto {
    GenericDataProto{
        version: generic_data.version,
        block_hash: generic_data.block_hash,
        block_number: generic_data.block_number,
        chain_type: generic_data.chain_type as i32,
        data_type: generic_data.data_type as i32,
        payload: generic_data.payload,
    }
}


fn _create_generic_block(   block_hash: String,
                            block:&Block) -> GenericData
{
    // Remove exstrinsics because cannot deserialize them. Maybe because of `Extrinsic: MaybeSerialize`
    // Todo: Deserialize exstrinsic
    let mut block = (*block).clone();
    //println!("Block content: {:?}",block);
    block.extrinsics = Vec::new();
    let generic_block = GenericData{
        chain_type: CHAIN_TYPE,
        version: VERSION.to_string(),
        data_type: DataType::Block,
        block_hash: block_hash,
        block_number: block.header.number as u64,
        payload: serde_json::to_vec(&block).unwrap()
    };
    // let decode_block: Block = serde_json::from_slice(&generic_block.payload).unwrap();
    // println!("decode_block: {:?}",tmp);
    generic_block
}

pub async fn get_data(mut chan: Arc<Mutex<BroadcastChannel<GenericDataProto>>>/*multiqueue::BroadcastSender<GenericDataProto>*/){

    println!("start");
    env_logger::init();
    let url = get_node_url_from_cli();


    let api = Api::<sr25519::Pair>::new(url).unwrap();

    // let head = api.get_finalized_head().unwrap().unwrap();
    //
    // println!("Finalized Head:\n {} \n", head);



    println!("Subscribing to finalized heads");
    let (send, recv) = channel();
    api.subscribe_finalized_heads(send).unwrap();

    loop{
        let head: Header = recv
            .recv()
            .map(|header| serde_json::from_str(&header).unwrap())
            .unwrap();
        // println!("Got new header {:?}", head);
        let generic_block = create_generic_block_from_header(&api, head).unwrap();
        println!("Got block number: {:?}, hash: {:?}", &generic_block.block_number,&generic_block.block_hash);
        let generic_block_proto = create_generic_data_proto_from_generic_data(generic_block);
        //println!("convert to proto block {:?}", &generic_block_proto);
        // Add to data list
        // {
        //      let mut lock_ls_generic_data = ls_generic_data.lock().await;
        //      lock_ls_generic_data.push(generic_block_proto);
        // };
        println!("Sending generic data");

        let chan_lock = chan.lock().await;
        chan_lock.send(&generic_block_proto).await;
        drop(chan_lock)

        //println!("Finish generic data {:?}",chan.next().await);
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
