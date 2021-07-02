use massbit_indexer::substrate_chain;
use tonic::{transport::Server};
use std::sync::Arc;
use tokio::sync::Mutex;
use massbit_indexer::stream_mod::streamout_server::{StreamoutServer};
use massbit_indexer::StreamService;
// use multiqueue;
// use broadcaster::BroadcastChannel;
// use futures_util::StreamExt;
use tokio::sync::broadcast;
const URL: &str = "127.0.0.1:50051";


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Queue
    let (chan, _) = broadcast::channel(16);
    let chan_sender = chan.clone();

    let stream_service = StreamService {
        chan
    };

    // spawm thread get_data
    let _ = tokio::spawn(async move {
        substrate_chain::get_data(chan_sender).await;
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

