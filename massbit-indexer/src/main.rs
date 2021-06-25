use massbit_indexer::substrate_chain;
use tonic::{transport::Server, Request, Response, Status};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;

pub mod stream_mod {
    tonic::include_proto!("streamout");
}

use stream_mod::streamout_server::{Streamout, StreamoutServer};
use stream_mod::{HelloReply, HelloRequest, GenericDataProto};

#[derive(Debug, Default)]
pub struct StreamService {
    ls_generic_data: Arc<Vec<GenericDataProto>>
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

    type ListFeaturesStream = ReceiverStream<Result<GenericDataProto, Status>>;

    async fn list_features(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::ListFeaturesStream>, Status> {
        println!("ListFeatures = {:?}", request);

        let (tx, rx) = mpsc::channel(4);
        let mut ls_generic_data = self.ls_generic_data.clone();

        tokio::spawn(async move {
            loop {
                if !ls_generic_data.is_empty(){
                    let generic_data = ls_generic_data.pop();
                    println!("  => send {:?}", generic_data.unwrap());
                    tx.send(Ok(generic_data.clone())).await.unwrap();
                }
            }

            println!(" /// done sending");
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }


}




#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse()?;

    let generic_data = GenericDataProto{
        version: "1".to_string()
    };
    let mut stream_service = StreamService {
        ls_generic_data: Arc::new(Vec::from([generic_data]))
    };

    stream_service.ls_generic_data.push(generic_data);

    Server::builder()
        .add_service(StreamoutServer::new(stream_service))
        .serve(addr)
        .await?;

    substrate_chain::get_data();
    Ok(())

}

