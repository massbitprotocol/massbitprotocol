use std::{thread, io};
use jsonrpc_http_server::{jsonrpc_core::{Compatibility, IoHandler, Params, Value}, ServerBuilder, jsonrpc_core};
use serde::{Deserialize};
use std::time::Duration;
use async_std::task;
use futures::executor::block_on;
use std::panic::AssertUnwindSafe;
// Massbit dependencies
use manifest_reader::core::load_file;

#[allow(unused_imports)]
use tonic::{transport::{Server, Channel}, Request, Response, Status};
use stream_mod::{HelloReply, HelloRequest, GetBlocksRequest, GenericDataProto};
use stream_mod::streamout_client::{StreamoutClient};
use std::error::Error;
use futures::channel::{mpsc, oneshot};
use futures::{SinkExt, StreamExt, Future, TryFutureExt};
use std::fmt::Debug;
use futures::future::FutureExt;
use std::sync::Arc;
use log::kv::ToValue;

#[derive(Clone, Debug, Deserialize)]
pub struct DeployParams {
    index_name: String,
    config_url: String,
}

#[allow(dead_code)]
pub struct JsonRpcServer {
    http_addr: String,
}

// This is a hack required because the json-rpc crate is not updated to tokio 0.2.
// We should watch the `jsonrpsee` crate and switch to that once it's ready.
async fn tokio02_spawn<I: Send + 'static, ER: Send + 'static>(
    mut task_sink: mpsc::Sender<Box<dyn std::future::Future<Output = ()> + Send + Unpin>>,
    future: impl std::future::Future<Output = Result<I, ER>> + Send + Unpin + 'static,
) -> Result<I, ER>
    where
        I: Debug,
        ER: Debug,
{
    let (return_sender, return_receiver) = oneshot::channel();
    task_sink
        .send(Box::new(future.map(move |res| {
            return_sender.send(res).expect("`return_receiver` dropped");
        })))
        .await
        .expect("task receiver dropped");
    return_receiver.await.expect("`return_sender` dropped")
}

fn abort_on_panic<T: Send + 'static>(
    f: impl Future<Output = T> + Send + 'static,
) -> impl Future<Output = T> {
    // We're crashing, unwind safety doesn't matter.
    AssertUnwindSafe(f).catch_unwind().unwrap_or_else(|_| {
        println!("Panic in tokio task, aborting!");
        std::process::abort()
    })
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
                // Blocking due to store interactions. Won't be blocking after #905.
                tokio::task::spawn_blocking(move || block_on(abort_on_panic(f)));
            }
        }));

        let sender = task_sender.clone();
        handler.add_method("index_deploy", move|params: Params| {
            // Use the graph tokio02 spawn
            Box::pin(tokio02_spawn(
                sender.clone(),
                async move {
                    let params = params.parse().unwrap();
                    deploy_handler(params).await
                }.boxed(),
            )).compat()
        });

        let server = ServerBuilder::new(handler)
            .start_http(&"127.0.0.1:3030".parse().unwrap())
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