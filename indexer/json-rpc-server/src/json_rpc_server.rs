use std::thread;
use jsonrpc_http_server::{
    jsonrpc_core::{self, Compatibility, IoHandler, Params, Value},
    RestApi, Server, ServerBuilder,
};
use serde::{Deserialize, Serialize};
use anyhow::anyhow;
use std::time::Duration;
use std::fs::File;

#[derive(Clone, Debug, Deserialize)]
struct DeployParams {
    index_name: String,
    config_url: String,
}

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
            println!("abc");
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

// Lazily load file from local
fn load_file(
    config_url: String,
) {
    let f = File::open(config_url).unwrap();
    let data: serde_yaml::Value = serde_yaml::from_reader(f).unwrap();

    let schemaFile = data["schema"]["file"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or(anyhow!("Could not find schema file"));
    println!("Schema: {}", schemaFile.unwrap()); // Refactor to use slog logger

    let kind = data["dataSources"][0]["kind"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or(anyhow!("Could not find network kind"));
    println!("Kind: {}", kind.unwrap()); // Refactor to use slog logger
}