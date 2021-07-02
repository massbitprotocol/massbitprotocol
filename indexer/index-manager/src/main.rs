// Massbit dependencies
use index_manager_server::types::JsonRpcServer;
use ipfs_client::core::create_ipfs_clients;
use logger::core::init_logger;

#[tokio::main]
async fn main() {
    // Logger
    init_logger();
    log::info!("[Indexer Manager] Application started");

    // Start IPFS Clients
    // Use lazy_static to put this ipfs_clients into global envs
    // let ipfs_addresses = vec!["0.0.0.0:5001".to_string()];
    // let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;
    // let file_bytes = ipfs_clients[0]
    //     .cat_all("QmZcB5qty5vGFRw2pEmwmyuyLxNbHM2dBaenNWj64JZ8uo".to_string())
    //     .compat()
    //     .await
    //     .unwrap()
    //     .to_vec();

    // let raw: serde_yaml::Mapping = serde_yaml::from_slice(&file_bytes).unwrap();

    // Start JSON RPC Server
    let server = JsonRpcServer::serve(
        "127.0.0.1:3030".to_string(),
    );
    server.wait();
}



