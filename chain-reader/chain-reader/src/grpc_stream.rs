use tokio::sync::{mpsc, broadcast};
use tokio_stream::wrappers::ReceiverStream;

use tonic::{Request, Response, Status};
use stream_mod::{ChainType, HelloReply, HelloRequest, GetBlocksRequest, GenericDataProto, streamout_server::Streamout};
use std::collections::HashMap;
use log::{debug, warn, error, info, Level};

pub mod stream_mod {
    tonic::include_proto!("chaindata");
}

#[derive(Debug)]
pub struct StreamService {
    pub chans: HashMap<ChainType,broadcast::Sender<GenericDataProto>>,
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

        // tx, rx for out stream gRPC
        let (tx, rx) = mpsc::channel(1024);

        // Create new channel for connect between input and output stream
        let mut rx_chan =  self.chans.get(&chain_type).unwrap().subscribe();

        tokio::spawn(async move {
            loop {
                // Getting generic_data
                let generic_data = rx_chan.recv().await.unwrap();
                // Send generic_data to queue"
                tx.send(Ok(generic_data)).await.unwrap();
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
