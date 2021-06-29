use std::thread;
use jsonrpc_http_server::{
    jsonrpc_core::{Compatibility, IoHandler, Params, Value},
    ServerBuilder,
};
use serde::{Deserialize};
use std::time::Duration;
// Massbit dependencies
use manifest_reader::core::load_file;

#[derive(Clone, Debug, Deserialize)]
struct DeployParams {
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
        // If we want to use tokio::spawn, need to grab the hackie code from the graph that resolve running tokio spawn with json_rpc_http_server
        // Reason: https://stackoverflow.com/questions/61292425/how-to-run-an-asynchronous-task-from-a-non-main-thread-in-tokio
        handler.add_method("index_deploy", |params: Params| {
            thread::spawn(|| {
                let params: DeployParams = params.parse().unwrap(); // Refactor to add param check
                println!("Received an index request from {}", params.index_name); // Refactor to use slog logger
                deploy_handler(params);
            });
            Ok(Value::String("Index deployed success".into()))
        });

        let server = ServerBuilder::new(handler)
            .start_http(&http_addr.parse().unwrap())
            .expect("Unable to start RPC server");
        server
    }
}

fn deploy_handler(
    params: DeployParams,
) {
    loop {
        load_file(params.config_url.clone()); // We are using loop for the indexing demo, but using clone is not efficient here
        thread::sleep(Duration::from_secs(1));
    }
}