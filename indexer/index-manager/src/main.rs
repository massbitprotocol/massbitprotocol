use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::thread;
use serde;
use serde::{Deserialize, Serialize};
use serde_json;
use slog::*;
use tokio::runtime::Runtime;

// Massbit dependencies
use json_rpc_server::json_rpc_server::JsonRpcServer;
use ipfs_client::ipfs_client::create_ipfs_clients;

#[tokio::main]
async fn main() {
    // Start JSON RPC Server
    let server = JsonRpcServer::serve(
        "127.0.0.1:3030".to_string(),
    );
    server.wait();

    // Start IPFS Clients
    let mut ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
    create_ipfs_clients(&ipfs_addresses).await;
}

