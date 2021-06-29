// Massbit dependencies
use json_rpc_server::core::JsonRpcServer;
use ipfs_client::core::create_ipfs_clients;
use logger::core::init_logger;

#[tokio::main]
async fn main() {
    // Logger
    init_logger();
    log::info!("Application started");

    // Start JSON RPC Server
    let server = JsonRpcServer::serve(
        "127.0.0.1:3030".to_string(),
    );
    server.wait();

    // Start IPFS Clients
    let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
    create_ipfs_clients(&ipfs_addresses).await; // This doesn't wait for connection
}



