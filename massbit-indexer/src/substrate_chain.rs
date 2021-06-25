use clap::App;

use sp_core::{sr25519, H256 as Hash};

use node_template_runtime::{Block, Header, SignedBlock};
use std::sync::mpsc::channel;
use substrate_api_client::Api;
use substrate_api_client::rpc::json_req;
use env_logger;
use serde_json;
use serde::{Serialize, Deserialize};
use std::error::Error;
use substrate_api_client::utils::FromHexString;

const CHAIN_TYPE: ChainType = ChainType::Substrate;
const VERSION:&str = "1";

#[derive(Debug)]
enum ChainType{
    Substrate,
    Ethereum,
    Solana,
}

#[derive(Debug)]
enum DataType{
    Block,
    Event,
    Transaction,
}
#[derive(Debug)]
struct GenericData{
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

fn _create_generic_block(   block_hash: String,
                            block:&Block) -> GenericData
{
    // Remove exstrinsics because cannot deserialize them. Maybe because of `Extrinsic: MaybeSerialize`
    // Todo: Deserialize exstrinsic
    let mut block = (*block).clone();
    block.extrinsics = Vec::new();
    let generic_block = GenericData{
        chain_type: CHAIN_TYPE,
        version: VERSION.to_string(),
        data_type: DataType::Block,
        block_hash: block_hash,
        block_number: block.header.number as u64,
        payload: serde_json::to_vec(&block).unwrap()
    };
    generic_block
}

pub fn get_data(){

    println!("start");
    env_logger::init();
    let url = get_node_url_from_cli();


    let api = Api::<sr25519::Pair>::new(url).unwrap();

    let head = api.get_finalized_head().unwrap().unwrap();

    println!("Finalized Head:\n {} \n", head);

    let h: Header = api.get_header(Some(head)).unwrap().unwrap();
    println!("Finalized header:\n {:?} \n", h);

    let b: SignedBlock = api.get_signed_block(Some(head)).unwrap().unwrap();
    println!("Finalized signed block:\n {:?} \n", b);

    println!(
        "Latest Header: \n {:?} \n",
        api.get_header::<Header>(None).unwrap()
    );

    println!("Subscribing to finalized heads");
    let (sender, receiver) = channel();
    api.subscribe_finalized_heads(sender).unwrap();

    loop{
        let head: Header = receiver
            .recv()
            .map(|header| serde_json::from_str(&header).unwrap())
            .unwrap();
        println!("Got new header {:?}", head);
        let generic_block = create_generic_block_from_header(&api, head);
        println!("Got new header {:?}", generic_block);
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
