use massbit_indexer::substrate_chain;
use tonic::{transport::Server};
use std::sync::Arc;
use tokio::sync::Mutex;
use massbit_indexer::stream_mod::streamout_server::{StreamoutServer};
use massbit_indexer::StreamService;
use multiqueue;
use broadcaster::BroadcastChannel;
use futures_util::StreamExt;
const URL: &str = "127.0.0.1:50051";


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Queue
    //let (send, first_recv) = multiqueue::broadcast_queue(3);
    //first_recv.unsubscribe();
    //let recv = first_recv.add_stream();
    //let arc_recv = Arc::new(Mutex::new(recv));
    // Create a Init StreamService struct
    let mut chan = BroadcastChannel::new();
    //let mut chan_send = chan.clone();
    let chan = Arc::new(Mutex::new(chan));
    let chan_send = chan.clone();
    let stream_service = StreamService {
        chan: chan
    };




    // spawm thread get_data
    let _ = tokio::spawn(async move {
        substrate_chain::get_data(chan_send).await;
    });

    // run StreamoutServer
    let addr = URL.parse()?;
    Server::builder()
        .add_service(StreamoutServer::new(stream_service))
        .serve(addr)
        .await?;

    // End
    Ok(())

}

