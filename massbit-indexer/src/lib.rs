#[macro_use]
extern crate clap;

pub mod substrate_chain;
pub mod stream_mod {
    tonic::include_proto!("streamout");
}

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tonic::{transport::Server, Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tokio::time::{sleep, Duration};
use crate::stream_mod::{HelloReply, HelloRequest, GenericDataProto};

#[derive(Debug, Default)]
pub struct StreamService {
    pub ls_generic_data: Arc<Mutex<Vec<GenericDataProto>>>
}

use stream_mod::streamout_server::{Streamout, StreamoutServer};

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

    type ListFeaturesStream = ReceiverStream<Result<GenericDataProto, Status>>;

    async fn list_features(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::ListFeaturesStream>, Status> {
        println!("ListFeatures = {:?}", request);

        let (tx, rx) = mpsc::channel(4);

        let mut ls_generic_data = Arc::clone(&self.ls_generic_data);

        tokio::spawn(async move {
            loop {
                let mut lock_ls_generic_data = ls_generic_data.lock().await;
                if !lock_ls_generic_data.is_empty(){
                    let generic_data = lock_ls_generic_data.pop().unwrap();
                    println!("  => send {:?}", &generic_data);
                    tx.send(Ok(generic_data.clone())).await.unwrap();
                }
                sleep(Duration::from_millis(100)).await;
            }

            println!(" /// done sending");
        });

        // let ls_generic_data = Arc::clone(&self.ls_generic_data);
        //
        // tokio::spawn(async move {
        //     let mut lock_ls_generic_data = ls_generic_data.lock().await;
        //     for generic_data in &lock_ls_generic_data[..] {
        //         println!("  => send {:?}", generic_data);
        //         tx.send(Ok(generic_data.clone())).await.unwrap();
        //     }
        //
        //     println!(" /// done sending");
        // });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}




// #[macro_use]
// extern crate dotenv;

// use dotenv::dotenv;
//use std::env;

