use serde_json;
use serde;
use serde::{Serialize, Deserialize};
use std::thread;
use std::time::Duration;
use jsonrpc_http_server::{
    jsonrpc_core::{self, Compatibility, IoHandler, Params, Value},
    RestApi, Server, ServerBuilder,
};
use std::sync::Arc;
use std::io;
use tokio::runtime::Runtime;

#[derive(Debug, Deserialize)]
struct IndexDeployParams {
    name: String,
    ipfs_hash: Option<String>,
    s3_link: Option<String>,
}

#[tokio::main]
async fn main() {
    let mut handler = IoHandler::default();

    // If we want to use tokio::spawn, need to grab the hackie code from the graph that resolve running tokio spawn with json_rpc_http_server
    // Reason: https://stackoverflow.com/questions/61292425/how-to-run-an-asynchronous-task-from-a-non-main-thread-in-tokio
    handler.add_method("index_deploy", |param: Params| {
        thread::spawn(|| {
            loop {
                println!("Received an index request");
                thread::sleep(Duration::from_secs(1));
            }
        });
        Ok(Value::String("hello".into()))
    });

    let server = ServerBuilder::new(handler)
        .start_http(&"127.0.0.1:3030".parse().unwrap())
        .expect("Unable to start RPC server");
    server.wait();


    // TODO: Refactor with JsonRpcServer later
    // let arc_self = Arc::new(JsonRpcServer {
    //     http_addr,
    // });
    // JsonRpcServer::serve(
    //     "127.0.0.1:3030".to_string(),
    // );
}


// TODO: Refactor with JsonRpcServer later
// pub struct JsonRpcServer {
//     http_addr: String,
// }
//
// impl JsonRpcServer {
//     fn serve(
//         http_addr: String,
//     ) -> Result<CustomServer, io::Error> {
//         let server = ServerBuilder::new(handler)
//             // .cors(DomainsValidation::AllowOnly(vec![AccessControlAllowOrigin::Null]))
//             .start_http(&http_addr.parse().unwrap())
//             .expect("Unable to start RPC server");
//         server
//     }
//
//     /// Handler for the `subgraph_deploy` endpoint.
//     async fn deploy_handler(
//         &self,
//         // params: SubgraphDeployParams,
//     ) -> Result<Value, jsonrpc_core::Error> {
//         Ok(serde_json::to_value("A").expect("invalid deploy"))
//     }
// }

