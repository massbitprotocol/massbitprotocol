#[allow(unused_imports)]
use tonic::{transport::{Server, Channel}, Request, Response, Status};
use crate::stream_mod::{HelloRequest, GetBlocksRequest, GenericDataProto, ChainType, DataType, streamout_client::StreamoutClient};
use std::error::Error;
use chain_reader::data_type::{SubstrateBlock as Block, SubstrateHeader as Header, SubstrateUncheckedExtrinsic as Extrinsic, decode_transactions};
use sp_core::{sr25519, H256 as Hash};
use node_template_runtime::Event;
use codec::{Decode, Encode};
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}
use chain_reader::data_type::decode;


type EventRecord = system::EventRecord<Event, Hash>;

const URL: &str = "http://127.0.0.1:50051";

pub async fn print_blocks(client: &mut StreamoutClient<Channel>, chain_type: ChainType) -> Result<(), Box<dyn Error>> {
    // Not use start_block_number start_block_number yet
    let get_blocks_request = GetBlocksRequest{
        start_block_number: 0,
        end_block_number: 1,
        chain_type: chain_type as i32,
    };
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();

    while let Some(data) = stream.message().await? {
        let mut data = data as GenericDataProto;
        println!("Recieved data block = {:?}, hash = {:?}, data type = {:?}",data.block_number, data.block_hash, DataType::from_i32(data.data_type).unwrap());
        //println!("Detail data block: {:?}", data);

        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                let block: Block = decode(&mut data.payload).unwrap();
                println!("Recieved BLOCK: {:?}", block.header.number);
            },
            Some(DataType::Event) => {
                let event: EventRecord = decode(&mut data.payload).unwrap();
                println!("Recieved EVENT: {:?}", event);
            },
            Some(DataType::Transaction) => {
                let extrinsics: Vec<Extrinsic> = decode_transactions(&mut data.payload).unwrap();
                println!("Recieved Extrinsic: {:?}", extrinsics);
            },

            _ => {
                println!("Not support data type: {:?}", &data.data_type);
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = StreamoutClient::connect(URL).await?;

    // Ping server
    println!("*** SIMPLE RPC ***");
    let response = client
        .say_hello(Request::new(HelloRequest {
            name: "new Client".to_string()
        }))
        .await?;

    println!("RESPONSE = {:?}", response);

    let chain_type = ChainType::Substrate;
    print_blocks(&mut client, chain_type).await?;

    Ok(())
}
