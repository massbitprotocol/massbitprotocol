use clap::App;

use sp_core::sr25519;

use node_template_runtime::{Block, Header, SignedBlock};
use std::sync::mpsc::channel;
use substrate_api_client::Api;
use env_logger;
use serde_json;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct MyBlock{
    block: Block
}


enum ChainType{
    Substrate,
    Ethereum,
}
enum DataType{
    Block,
    Event,
    Transaction,
}

struct GenericData{
    chain_type: ChainType,
    data_type: DataType,
    version: String,
    block_hash: String,
    time_stamp: String,
    payload: Vec<u8>
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

    let wrap_block = MyBlock{ block:api.get_block::<Block>(None).unwrap().unwrap()};
    println!(
        "Latest block: \n {:?} \n",
        wrap_block
    );


    let serialized = serde_json::to_string(&wrap_block).unwrap();
    println!("serialized = {:?}", &serialized);

    // let deserialized: MyBlock = serde_json::from_str(&serialized).unwrap();
    // println!("deserialized = {:?}", deserialized);

    println!("Subscribing to finalized heads");
    let (sender, receiver) = channel();
    api.subscribe_finalized_heads(sender).unwrap();

    loop{
        let head: Header = receiver
            .recv()
            .map(|header| serde_json::from_str(&header).unwrap())
            .unwrap();
        println!("Got new Block {:?}", head);
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
