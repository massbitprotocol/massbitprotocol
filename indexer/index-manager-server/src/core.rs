use jsonrpc_http_server::{jsonrpc_core::{Compatibility, IoHandler, Params, Value}, ServerBuilder, jsonrpc_core};
use serde::{Deserialize};
use futures::channel::{mpsc};
use futures::executor::block_on;
use futures::{StreamExt, TryFutureExt};
use futures::future::FutureExt;
use std::{path::PathBuf};

// Massbit dependencies
use manifest_reader::core::load_file;
use tokio02_spawn::core::abort_on_panic;
use tokio02_spawn::core::tokio02_spawn;
use plugin::manager::PluginManager;
use massbit_chain_substrate::data_type::SubstrateBlock;

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
        // Config Server to run with tokio02
        let mut handler = IoHandler::with_compatibility(Compatibility::Both);
        let (task_sender, task_receiver) =
            mpsc::channel::<Box<dyn std::future::Future<Output = ()> + Send + Unpin>>(100);
            tokio::spawn(task_receiver.for_each(|f| {
            async {
                tokio::task::spawn_blocking(move || block_on(abort_on_panic(f)));
            }
        }));
        let sender_local = task_sender.clone();
        let sender_ipfs = task_sender.clone();

        //
        // All Handlers Mapping
        //
        handler.add_method("index_deploy_local", move|params: Params| {
            Box::pin(tokio02_spawn(
                sender_local.clone(),
                async move {
                    let params = params.parse().unwrap();
                    // Add function: call to local folder to get config (.yaml) / SO rust file
                    deploy_local_handler(params).await
                }.boxed(),
            )).compat()
        });

        handler.add_method("index_deploy_ipfs", move|params: Params| {
            Box::pin(tokio02_spawn(
                sender_ipfs.clone(),
                async move {
                    let params = params.parse().unwrap();
                    // Add function: call to IPFS to get config (.yaml) / SO rust file
                    deploy_ipfs_handler(params).await
                }.boxed(),
            )).compat()
        });

        // Start the server
        let server = ServerBuilder::new(handler)
            .start_http(&http_addr.parse().unwrap())
            .expect("Unable to start RPC server");
        server
    }
}

//
// All Handlers
//
async fn deploy_local_handler(
    params: DeployParams,
) -> Result<Value, jsonrpc_core::Error> {
    tokio::spawn(async move{
        let mut client = StreamoutClient::connect(URL).await.unwrap();
        print_blocks(&mut client, params).await; // Start Chain Reader Client
    });
    Ok(serde_json::to_value("Deployed index from local success").expect("Unable to create Index"))
}

async fn deploy_ipfs_handler(
    params: DeployParams,
) -> Result<Value, jsonrpc_core::Error> {
    tokio::spawn(async move{
        let mut client = StreamoutClient::connect(URL).await.unwrap();
        print_blocks(&mut client, params).await; // Start Chain Reader Client
    });
    Ok(serde_json::to_value("Deployed index from ipfs success").expect("Unable to create Index"))
}

//
// Chain reader client. Migrate to a new cargo
//
#[allow(unused_imports)]
use tonic::{transport::{Server, Channel}, Request, Response, Status};
use stream_mod::{GetBlocksRequest, GenericDataProto};
use stream_mod::streamout_client::{StreamoutClient};
use std::error::Error;
use std::fs::File;

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

    // Start Plugin Manager
    println!("Start plugin manager");
    // The main loop, subscribing to Chain Reader Server to get new block
    while let Some(block) = stream.message().await? {
        let block = block as GenericDataProto;
        println!("Recieved block = {:?}, hash = {:?} from {:?}",block.block_number, block.block_hash, params.index_name);
        let library_path = PathBuf::from("./target/release/libblock.so".to_string());
        let mut plugins = PluginManager::new();
        unsafe {
            plugins
                .load(&library_path)
                .expect("plugin loading failed");
        }

        let decode_block: SubstrateBlock = serde_json::from_slice(&block.payload).unwrap();
        println!("decode_block: {:?}",decode_block);
        plugins.handle_block(&decode_block); // Block handling
    }
    Ok(())
}
