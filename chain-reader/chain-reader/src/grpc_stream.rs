use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use crate::ethereum_chain;
use crate::CONFIG;
use log::{error, info};
use std::collections::HashMap;
use stream_mod::{
    streamout_server::Streamout, ChainType, GenericDataProto, GetBlocksRequest, HelloReply,
    HelloRequest,
};
use tonic::{Request, Response, Status};

const QUEUE_BUFFER: usize = 1024;

pub mod stream_mod {
    tonic::include_proto!("chaindata");
}

#[derive(Debug)]
pub struct StreamService {
    pub chans: HashMap<ChainType, broadcast::Sender<GenericDataProto>>,
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
        //let chain_type: ChainType = ChainType::from_i32(request.get_ref().chain_type).unwrap();
        let start_block = request.get_ref().start_block_number;
        let (tx, rx) = mpsc::channel(QUEUE_BUFFER);

        tokio::spawn(async move {
            let start_block = match start_block {
                0 => None,
                _ => Some(start_block),
            };
            let resp = ethereum_chain::loop_get_block(tx.clone(), &start_block).await;

            error!("Stop loop_get_block, error: {:?}", resp);
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
