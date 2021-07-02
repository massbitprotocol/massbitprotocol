#[macro_use]
extern crate clap;

pub mod substrate_chain;
pub mod stream_mod {
    tonic::include_proto!("streamout");
}

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio::time::{sleep, Duration};

use tonic::{Request, Response, Status};
use crate::stream_mod::{HelloReply, HelloRequest, GetBlocksRequest, GenericDataProto};
use stream_mod::streamout_server::{Streamout};
//use multiqueue;
// use broadcaster::BroadcastChannel;
// use futures_util::StreamExt;
use tokio::sync::broadcast;


#[derive(Debug)]
pub struct StreamService {
    pub chan : Arc<Mutex<broadcast::Sender<GenericDataProto>>>,
}


#[tonic::async_trait]
impl Streamout for StreamService {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {:?}", request);

        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name).into(),
        };

        Ok(Response::new(reply))
    }

    type ListBlocksStream = ReceiverStream<Result<GenericDataProto, Status>>;

    async fn list_blocks(
        &self,
        request: Request<GetBlocksRequest>,
    ) -> Result<Response<Self::ListBlocksStream>, Status> {
        println!("ListFeatures = {:?}", request);

        // tx, rx for out stream gRPC
        let (tx, rx) = mpsc::channel(4);

        // Create new channel for connect between input and output stream
        let mut lock_chan = self.chan.lock().await;
        let mut new_chain =  lock_chan.clone();
        drop(lock_chan);

        tokio::spawn(async move {
            loop {
                println!("Getting generic_data");
                let generic_data = new_chain.next().await;
                println!("Send generic_data to queue");
                tx.send(Ok(generic_data.unwrap())).await.unwrap();
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}



