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

use tonic::{transport::Server, Request, Response, Status};
use crate::stream_mod::{HelloReply, HelloRequest, GenericDataProto};
use stream_mod::streamout_server::{Streamout, StreamoutServer};


#[derive(Debug, Default)]
pub struct StreamService {
    pub ls_generic_data: Arc<Mutex<Vec<GenericDataProto>>>
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

    type StreamBlockStream = ReceiverStream<Result<GenericDataProto, Status>>;

    async fn stream_blocks(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::ListFeaturesStream>, Status> {
        println!("ListFeatures = {:?}", request);

        let (tx, rx) = mpsc::channel(4);

        let mut ls_generic_data = Arc::clone(&self.ls_generic_data);

        tokio::spawn(async move {
            // Todo: Need improve because of the locking time and pop effect.
            loop {
                // lock data block
                {
                    let mut lock_ls_generic_data = ls_generic_data.lock().await;
                    if !lock_ls_generic_data.is_empty() {
                        let generic_data = lock_ls_generic_data.pop().unwrap();
                        println!("  => send {:?}", &generic_data);
                        tx.send(Ok(generic_data.clone())).await.unwrap();
                    }
                }
                sleep(Duration::from_millis(300)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}



