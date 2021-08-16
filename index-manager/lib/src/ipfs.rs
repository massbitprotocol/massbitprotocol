use lazy_static::lazy_static;
use std::fs::File;
use std::io::Read;
use std::{env, fs};
/**
*** Objective of this file is to call to IPFS and get the index's information
**/
// Generic dependencies
use tokio_compat_02::FutureExt;

// Massbit dependencies
use ipfs_client::core::create_ipfs_clients;
use std::path::PathBuf;

lazy_static! {
    static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
    static ref GENERATED_FOLDER: String = String::from("index-manager/generated/");
}

pub async fn get_ipfs_file_by_hash(file: &String, hash: &String) -> String {
    log::info!("Downloading {} from IPFS as {}", hash, file);
    let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    let ipfs_clients = create_ipfs_clients(&ipfs_addresses).await;
    let file_bytes = ipfs_clients[0]
        .cat_all(hash.to_string())
        .compat()
        .await
        .unwrap()
        .to_vec();

    let file_path = [GENERATED_FOLDER.as_str(), file].join("");
    let res = fs::write(file_path.clone(), file_bytes);
    match res {
        Ok(_) => {
            log::info!("Write {} to storage successfully", file_path);
            file_path
        }
        Err(err) => {
            panic!("Could not write {} to storage {:#?}", file, err)
        }
    }
}

pub fn read_config_file(config: &PathBuf) -> serde_yaml::Value {
    let mut project_config_string = String::new();
    let mut f = File::open(config).expect("Unable to open file"); // Refactor: Config to download config file from IPFS instead of just reading from local
    f.read_to_string(&mut project_config_string)
        .expect("Unable to read string"); // Get raw query
    let project_config: serde_yaml::Value = serde_yaml::from_str(&project_config_string).unwrap();
    project_config
}
