extern crate jsonrpc_http_server;
extern crate lazy_static;
extern crate massbit;
extern crate serde;

use self::massbit::data::indexer::IndexerRegistrarError;
use futures::StreamExt;
use jsonrpc_http_server::{
    jsonrpc_core::{self, Compatibility, IoHandler, Params, Value},
    RestApi, Server, ServerBuilder,
};
use lazy_static::lazy_static;
use massbit::prelude::futures03::channel::{mpsc, oneshot};
use massbit::prelude::futures03::SinkExt;
use massbit::prelude::{JsonRpcServer as JsonRpcServerTrait, *};
use std::env;
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4};

lazy_static! {
    static ref EXTERNAL_HTTP_BASE_URL: Option<String> = env::var_os("EXTERNAL_HTTP_BASE_URL")
        .map(|s| s.into_string().expect("invalid external HTTP base URL"));
    static ref EXTERNAL_WS_BASE_URL: Option<String> = env::var_os("EXTERNAL_WS_BASE_URL")
        .map(|s| s.into_string().expect("invalid external WS base URL"));
}

const JSON_RPC_DEPLOY_ERROR: i64 = 0;

#[derive(Debug, Deserialize)]
struct IndexerDeployParams {
    name: IndexerName,
    ipfs_hash: DeploymentHash,
}

pub struct JsonRpcServer<R> {
    registrar: Arc<R>,
    logger: Logger,
}

impl<R: IndexerRegistrar> JsonRpcServer<R> {
    async fn deploy_handler(
        &self,
        params: IndexerDeployParams,
    ) -> Result<Value, jsonrpc_core::Error> {
        info!(&self.logger, "Received indexer_deploy request"; "params" => format!("{:?}", params));

        match self
            .registrar
            .create_indexer(params.name.clone(), params.ipfs_hash.clone())
            .await
        {
            Ok(_) => Ok(Value::Null),
            Err(e) => Err(json_rpc_error(
                &self.logger,
                "indexer_deploy",
                e,
                JSON_RPC_DEPLOY_ERROR,
                params,
            )),
        }
    }
}

impl<R> JsonRpcServerTrait<R> for JsonRpcServer<R>
where
    R: IndexerRegistrar,
{
    type Server = Server;

    fn serve(port: u16, registrar: Arc<R>, logger: Logger) -> Result<Self::Server, io::Error> {
        let logger = logger.new(o!("component" => "IndexerManager"));

        info!(
            logger,
            "Starting JSON-RPC indexer manager server at: http://localhost:{}", port
        );

        let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);

        let mut handler = IoHandler::with_compatibility(Compatibility::Both);

        let arc_self = Arc::new(JsonRpcServer { registrar, logger });

        let (task_sender, task_receiver) =
            mpsc::channel::<Box<dyn std::future::Future<Output = ()> + Send + Unpin>>(100);
        massbit::spawn(task_receiver.for_each(|f| {
            async {
                // Blocking due to store interactions. Won't be blocking after #905.
                massbit::spawn_blocking(f);
            }
        }));

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

        let me = arc_self.clone();
        let sender = task_sender.clone();
        handler.add_method("subgraph_deploy", move |params: Params| {
            let me = me.clone();
            Box::pin(tokio02_spawn(
                sender.clone(),
                async move {
                    let params = params.parse()?;
                    me.deploy_handler(params).await
                }
                .boxed(),
            ))
            .compat()
        });

        ServerBuilder::new(handler)
            // Enable REST API:
            // POST /<method>/<param1>/<param2>
            .rest_api(RestApi::Secure)
            .start_http(&addr.into())
    }
}

fn json_rpc_error(
    logger: &Logger,
    operation: &str,
    e: IndexerRegistrarError,
    code: i64,
    params: impl std::fmt::Debug,
) -> jsonrpc_core::Error {
    error!(logger, "{} failed", operation;
        "error" => format!("{:?}", e),
        "params" => format!("{:?}", params));

    let message = if let IndexerRegistrarError::Unknown(_) = e {
        "internal error".to_owned()
    } else {
        e.to_string()
    };

    jsonrpc_core::Error {
        code: jsonrpc_core::ErrorCode::ServerError(code),
        message,
        data: None,
    }
}
