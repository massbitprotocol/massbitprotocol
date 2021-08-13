// Massbit dependencies
use index_manager_lib::index_manager::IndexManager;
use logger::core::init_logger;

#[tokio::main]
async fn main() {
    let res = init_logger(&String::from("index-manager"));
    println!("{}", res);
    IndexManager::restart_all_existing_index().await;
    let server = IndexManager::serve("0.0.0.0:3030".to_string());
    server.wait();
}
