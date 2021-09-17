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

        let (tx, rx) = mpsc::channel(1024);

        tokio::spawn(async move {
            let mut count = 1;
            let mut got_block_number = CONFIG.chains.get(&chain_type).unwrap().start_block;

            loop {
                let resp = ethereum_chain::loop_get_block(tx.clone(), &mut got_block_number).await;
                error!(
                    "Restart {:?} response {:?}, at block {:?}, {} time",
                    &chain_type, resp, &got_block_number, count
                );
                count = count + 1;
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
