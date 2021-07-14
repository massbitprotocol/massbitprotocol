#[allow(unused_imports)]
use tonic::{transport::{Server, Channel}, Request, Response, Status};
use crate::stream_mod::{HelloRequest, GetBlocksRequest, GenericDataProto, ChainType, DataType, streamout_client::StreamoutClient};
use std::error::Error;
use massbit_chain_substrate::data_type::{
    SubstrateBlock, SubstrateHeader, SubstrateUncheckedExtrinsic, decode_transactions};
use massbit_chain_solana::data_type::{
    SolanaBlock, decode as solana_decode
    };

use sp_core::{sr25519, H256 as Hash};
use node_template_runtime::Event;
use codec::{Decode, Encode};
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}
use massbit_chain_substrate::data_type::decode;
use std::sync::Arc;


type EventRecord = system::EventRecord<Event, Hash>;

const URL: &str = "http://127.0.0.1:50051";

pub async fn print_blocks(mut client: StreamoutClient<Channel>, chain_type: ChainType) -> Result<(), Box<dyn Error>> {
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
        println!("Recieved chain: {:?}, data block = {:?}, hash = {:?}, data type = {:?}",
                 ChainType::from_i32(data.chain_type).unwrap(),
                 data.block_number,
                 data.block_hash,
                 DataType::from_i32(data.data_type).unwrap());
        //println!("Detail data block: {:?}", data);

        match chain_type {
            ChainType::Substrate => {
                match DataType::from_i32(data.data_type) {
                    Some(DataType::Block) => {
                        let block: SubstrateBlock = decode(&mut data.payload).unwrap();
                        println!("Recieved BLOCK: {:?}", block.header.number);
                    },
                    Some(DataType::Event) => {
                        let event: EventRecord = decode(&mut data.payload).unwrap();
                        println!("Recieved EVENT: {:?}", event);
                    },
                    Some(DataType::Transaction) => {
                        let extrinsics: Vec<SubstrateUncheckedExtrinsic> = decode_transactions(&mut data.payload).unwrap();
                        println!("Recieved Extrinsic: {:?}", extrinsics);
                    },

                    _ => {
                        println!("Not support data type: {:?}", &data.data_type);
                    }
                }
            },
            ChainType::Solana => {
                match DataType::from_i32(data.data_type) {
                    Some(DataType::Block) => {
                        //println!("Recieved data: {:?}", data);
                        let block: SolanaBlock = solana_decode(&mut data.payload).unwrap();
                        println!("Recieved BLOCK with block height: {:?}, hash: {:?}", &block.block_height.unwrap(), &block.blockhash);

                    },
                    _ => {
                        println!("Not support type in Solana");
                    }
                }
            },
            _ => {
                println!("Not support this package chain-type");
            }



        }

    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    println!("Waiting for chain-reader");

    tokio::spawn(async move {
        let mut client = StreamoutClient::connect(URL).await.unwrap();
        print_blocks(client, ChainType::Solana).await;
    });

    let mut client = StreamoutClient::connect(URL).await.unwrap();
    print_blocks(client, ChainType::Substrate).await;



    Ok(())
}
