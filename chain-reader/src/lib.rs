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
use broadcaster::BroadcastChannel;
use futures_util::StreamExt;


#[derive(Debug)]
pub struct StreamService {
    //pub receiver : Arc<Mutex<multiqueue::BroadcastReceiver<GenericDataProto>>>,
    pub chan : Arc<Mutex<BroadcastChannel<GenericDataProto>>>,
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

        let (tx, rx) = mpsc::channel(4);

        //let ls_generic_data = Arc::clone(&self.ls_generic_data);
        // let mut lock_receiver = self.receiver.lock().await;
        // let recv = lock_receiver.add_stream();
        println!("Wait chan lock");
        let mut lock_chan = self.chan.lock().await;
        println!("chan is locked");
        let mut new_chain =  lock_chan.clone();
        drop(lock_chan);

        tokio::spawn(async move {
            // Todo: Need improve because of the locking time and pop effect.
            //for generic_data in lock_chan {
                // lock data block
            loop{
                {
                    println!("Getting generic_data");
                    //println!("Got generic data {:?}",new_chain.next().await);
                    let generic_data = new_chain.next().await;
                    //let mut lock_ls_generic_data = ls_generic_data.lock().await;
                    //if !lock_ls_generic_data.is_empty() {

                        //let generic_data = lock_ls_generic_data.pop().unwrap();
                        //println!("  => send block: {:?}, hash: {:?}", &generic_data.block_number, &generic_data.block_hash);
                        tx.send(Ok(generic_data.unwrap()/*.clone()*/)).await.unwrap();
                    //}
                }
                //println!("Wait for next data");
                //sleep(Duration::from_millis(300)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}



