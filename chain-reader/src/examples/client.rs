#[allow(unused_imports)]
use tonic::{transport::{Server, Channel}, Request, Response, Status};
use crate::stream_mod::{HelloRequest, GetBlocksRequest, GenericDataProto, streamout_client::StreamoutClient};
use std::error::Error;
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}

const URL: &str = "http://127.0.0.1:50051";

pub async fn print_blocks(client: &mut StreamoutClient<Channel>) -> Result<(), Box<dyn Error>> {
    // Not use start_block_number start_block_number yet
    let get_blocks_request = GetBlocksRequest{
        start_block_number: 0,
        end_block_number: 1,
    };
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();

    while let Some(block) = stream.message().await? {
        let block = block as GenericDataProto;
        println!("Recieved block = {:?}, hash = {:?}",block.block_number, block.block_hash);
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

    print_blocks(&mut client).await?;

    Ok(())
}
