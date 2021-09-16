use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use crate::command::NetworkType;
use log::{error, info};
use std::collections::HashMap;
use stream_mod::{
    streamout_server::Streamout, ChainType, GenericDataProto, GetBlocksRequest, HelloReply,
    HelloRequest,
};
use tonic::{Request, Response, Status};

pub mod stream_mod {
    tonic::include_proto!("chaindata");
}

#[derive(Debug)]
pub struct StreamService {
    pub chans: HashMap<(ChainType, NetworkType), broadcast::Sender<GenericDataProto>>,
}

#[tonic::async_trait]
impl Streamout for StreamService {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        info!("Got a request: {:?}", request);

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
        info!("Request = {:?}", request);
        let chain_type: ChainType = ChainType::from_i32(request.get_ref().chain_type).unwrap();
        let network: NetworkType = request.get_ref().network.clone();

        // tx, rx for out stream gRPC
        let (tx, rx) = mpsc::channel(1024);

        // Create new channel for connect between input and output stream
        println!(
            "chains: {:?}, chain_type: {:?}, network: {}",
            &self.chans, chain_type, network
        );

        let mut rx_chan = self.chans.get(&(chain_type, network)).unwrap().subscribe();

        tokio::spawn(async move {
            loop {
                // Getting generic_data
                let generic_data = rx_chan.recv().await.unwrap();
                // Send generic_data to queue"
                let res = tx.send(Ok(generic_data)).await;
                if res.is_err() {
                    error!("Cannot send data to RPC client queue, error: {:?}", res);
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
