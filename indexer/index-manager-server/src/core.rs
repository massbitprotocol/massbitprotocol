use jsonrpc_http_server::{jsonrpc_core::{Compatibility, IoHandler, Params, Value}, ServerBuilder, jsonrpc_core};
use serde::{Deserialize};
use futures::channel::{mpsc};
use futures::executor::block_on;
use futures::{StreamExt, TryFutureExt};
use futures::future::FutureExt;

// Massbit dependencies
use manifest_reader::core::load_file;
use tokio02_spawn::core::abort_on_panic;
use tokio02_spawn::core::tokio02_spawn;

#[derive(Clone, Debug, Deserialize)]
pub struct DeployParams {
    index_name: String,
    config_url: String,
}

#[allow(dead_code)]
pub struct JsonRpcServer {
    http_addr: String,
}

impl JsonRpcServer {
    pub fn serve(
        http_addr: String,
    ) -> jsonrpc_http_server::Server {

        let mut handler = IoHandler::with_compatibility(Compatibility::Both);
        let (task_sender, task_receiver) =
            mpsc::channel::<Box<dyn std::future::Future<Output = ()> + Send + Unpin>>(100);
            tokio::spawn(task_receiver.for_each(|f| {
            async {
                tokio::task::spawn_blocking(move || block_on(abort_on_panic(f)));
            }
        }));

        let sender = task_sender.clone();
        handler.add_method("index_deploy", move|params: Params| {
            Box::pin(tokio02_spawn(
                sender.clone(),
                async move {
                    let params = params.parse().unwrap();
                    deploy_handler(params).await
                }.boxed(),
            )).compat()
        });

        let server = ServerBuilder::new(handler)
            .start_http(&http_addr.parse().unwrap())
            .expect("Unable to start RPC server");
        server
    }
}

async fn deploy_handler(
    params: DeployParams,
) -> Result<Value, jsonrpc_core::Error> {
    tokio::spawn(async move{
        let mut client = StreamoutClient::connect(URL).await.unwrap();
        print_blocks(&mut client, params).await;
    });
    Ok(serde_json::to_value("Deployed Index Success").expect("Unable to create Index"))
}

//
// Chain reader client. Migrate to a new cargo
//
#[allow(unused_imports)]
use tonic::{transport::{Server, Channel}, Request, Response, Status};
use stream_mod::{GetBlocksRequest, GenericDataProto};
use stream_mod::streamout_client::{StreamoutClient};
use std::error::Error;
pub mod stream_mod {
    tonic::include_proto!("streamout");
}
const URL: &str = "http://127.0.0.1:50051";
pub async fn print_blocks(client: &mut StreamoutClient<Channel>, params: DeployParams) -> Result<(), Box<dyn Error>> {
    // Not use start_block_number start_block_number yet
    let get_blocks_request = GetBlocksRequest{
        start_block_number: 0,
        end_block_number: 1,
    };
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();

    while let Some(block) = stream.message().await? {
        let block = block as GenericDataProto;
        println!("Recieved block = {:?}, hash = {:?} from {:?}",block.block_number, block.block_hash, params.index_name);
    }
    Ok(())
}