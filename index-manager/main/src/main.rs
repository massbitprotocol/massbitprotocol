use lazy_static::lazy_static;
/**
 *** Objective of this file is to  
 *** - start a JSON HTTP SERVER to receive the index requests and expose some API about indexers
 *** - support restarting indexers
 **/
// Generic dependencies
use std::env;

// Massbit dependencies
use index_manager_lib::index_manager::IndexManager;
use logger::core::init_logger;

lazy_static! {
    // Restart all the indexes when the indexer manager is restarted is still a new feature.
    // We don't want it to be broken when running the E2E Tests, so default option is set to False
    static ref INDEX_MANAGER_RESTART_INDEX: String = env::var("INDEX_MANAGER_RESTART_INDEX").unwrap_or(String::from("false"));
}

#[tokio::main]
async fn main() {
    let res = init_logger(&String::from("index-manager"));
    println!("{}", res); // Print log output type
    IndexManager::run_migration();
    if INDEX_MANAGER_RESTART_INDEX.to_lowercase().as_str() == "true" {
        tokio::spawn(async move {
            IndexManager::restart_all_existing_index().await;
        });
    }

    let server = IndexManager::serve("0.0.0.0:3030".to_string());
    server.wait();
}
