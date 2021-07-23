use jsonrpc_http_server::{jsonrpc_core::{Compatibility, IoHandler, Params, Value}, ServerBuilder, jsonrpc_core};
use futures::channel::{mpsc};
use futures::executor::block_on;
use futures::{StreamExt, TryFutureExt};
use futures::future::FutureExt;

// Massbit dependencies
use tokio02_spawn::core::abort_on_panic;
use tokio02_spawn::core::tokio02_spawn;
use crate::helper::{loop_blocks, list_handler_helper};
use crate::types::{IndexManager, DeployParams};

impl IndexManager {
    pub fn serve(
        http_addr: String,
    ) -> jsonrpc_http_server::Server {
        // Use mpsc channel to spawn tokio 0.2 because json-rpc-http-server crate is not updated to tokio 0.2
        let mut handler = IoHandler::with_compatibility(Compatibility::Both);
        let (task_sender, task_receiver) =
            mpsc::channel::<Box<dyn std::future::Future<Output = ()> + Send + Unpin>>(100);
        tokio::spawn(task_receiver.for_each(|f| {
            async {
                tokio::task::spawn_blocking(move || block_on(abort_on_panic(f)));
            }
        }));
        let sender_deploy = task_sender.clone();
        let sender_list = task_sender.clone();

        handler.add_method("index_list", move |_| {
            Box::pin(tokio02_spawn(
                sender_list.clone(),
                async move {
                    list_handler().await
                }.boxed(),
            )).compat()
        });

        handler.add_method("index_deploy", move|params: Params| {
            Box::pin(tokio02_spawn(
                sender_deploy.clone(),
                async move {
                    let params = params.parse().unwrap();
                    deploy_handler(params).await
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

async fn deploy_handler(
    params: DeployParams,
) -> Result<Value, jsonrpc_core::Error> {
    #[allow(unused_must_use)]
        tokio::spawn(async move{
        loop_blocks(params).await;// Start streaming and indexing blocks
    });
    Ok(serde_json::to_value("Deploy index success").expect("Unable to deploy new index"))
}

async fn list_handler(
) -> Result<Value, jsonrpc_core::Error> {
    let indexers = list_handler_helper().await.unwrap();
    Ok(serde_json::to_value(indexers).expect("Unable to get index list"))
}
