// Massbit dependencies
use index_manager_lib::index_manager::IndexManager;
use logger::core::init_logger;

#[tokio::main]
async fn main() {
    // Logger
    init_logger(&String::from("index-manager"));
    log::info!("Application started");

    // Start Index Manager Server
    let server = IndexManager::serve("0.0.0.0:3030".to_string());
    server.wait();
}
