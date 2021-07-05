// Massbit dependencies
use index_manager_server::types::JsonRpcServer;
use logger::core::init_logger;

#[tokio::main]
async fn main() {
    // Logger
    init_logger();
    log::info!("[Indexer Manager] Application started");

    // Start JSON RPC Server
    let server = JsonRpcServer::serve(
        "127.0.0.1:3030".to_string(),
    );
    server.wait();
}



