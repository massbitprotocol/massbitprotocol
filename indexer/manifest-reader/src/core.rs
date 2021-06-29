use thiserror::Error;
use std::fs::File;
use anyhow::anyhow;

#[derive(Error, Debug)]
pub enum ManifestValidationError {
    #[error("manifest has no data sources")]
    NoDataSources,
    #[error("manifest cannot index data from different networks")]
    MultipleNetworks,
}

// Lazily load file from local
pub fn load_file(
    config_url: String,
) {
    let f = File::open(config_url).unwrap();
    let data: serde_yaml::Value = serde_yaml::from_reader(f).unwrap();

    let schema = data["schema"]["file"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or(anyhow!("Could not find schema file"));
    println!("Schema: {}", schema.unwrap()); // Refactor to use slog logger

    let kind = data["dataSources"][0]["kind"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or(anyhow!("Could not find network kind"));
    println!("Kind: {}", kind.unwrap()); // Refactor to use slog logger
}