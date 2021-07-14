// Massbit dependencies
use index_manager::types::IndexManager;
use logger::core::init_logger;

#[tokio::main]
async fn main() {
    // Logger
    init_logger();
    log::info!("[Indexer Manager] Application started");

    // Start Index Manager Server
    let server = IndexManager::serve(
        "0.0.0.0:3030".to_string(),
    );
    server.wait();
}



