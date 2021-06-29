use massbit_indexer::substrate_chain;
use tonic::{transport::Server, Request, Response, Status};
use std::sync::Arc;
use tokio::sync::Mutex;
use massbit_indexer::stream_mod::streamout_server::{Streamout, StreamoutServer};
use massbit_indexer::stream_mod::{HelloReply, s, GenericDataProto};
use massbit_indexer::StreamService;
const url : &str = "127.0.0.1:50051";


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a Init StreamService struct
    let mut stream_service = StreamService {
        ls_generic_data: Arc::new(Mutex::new(Vec::from([])))
    };

    // Clone the Vec of block data
    let ls_generic_data = Arc::clone(&stream_service.ls_generic_data);


    // spawm thread get_data
    let thread_get_data = tokio::spawn(async move {
        substrate_chain::get_data(ls_generic_data).await;
    });

    // run StreamoutServer
    let addr = url.parse()?;
    Server::builder()
        .add_service(StreamoutServer::new(stream_service))
        .serve(addr)
        .await?;

    // End
    Ok(())

}

