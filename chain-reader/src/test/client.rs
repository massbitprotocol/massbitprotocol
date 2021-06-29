#[allow(unused_imports)]
use tonic::{transport::Server, Request, Response, Status};
use crate::stream_mod::{HelloReply, HelloRequest, GenericDataProto};
use stream_mod::streamout_client::{StreamoutClient};

pub mod stream_mod {
    tonic::include_proto!("streamout");
}

const URL: &str = "127.0.0.1:50051";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = StreamoutClient::connect(URL).await?;

    println!("*** SIMPLE RPC ***");
    let response = client
        .say_hello(Request::new(HelloRequest {
            name: "new Client".to_string()
        }))
        .await?;

    println!("RESPONSE = {:?}", response);

    // println!("\n*** SERVER STREAMING ***");
    // print_features(&mut client).await?;
    //
    // println!("\n*** CLIENT STREAMING ***");
    // run_record_route(&mut client).await?;
    //
    // println!("\n*** BIDIRECTIONAL STREAMING ***");
    // run_route_chat(&mut client).await?;

    Ok(())
}
