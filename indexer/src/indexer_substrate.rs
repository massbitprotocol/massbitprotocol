use postgres::{Client as PostgreClient, NoTls};
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Deserialize;
use serde_json::json;
use std::error::Error;
use std::hash::Hash;
use reqwest::Client;
use tokio_compat_02::FutureExt;
use std::thread;
use std::time::Duration;

#[derive(Deserialize, Debug)]
struct BlockResult {
    id: u64,
    jsonrpc: String,
    result: String,
}

#[derive(Clone)]
pub struct IndexerSubstrate {
}

impl IndexerSubstrate {
    pub async fn start_index() {
        let mut client =
            PostgreClient::connect("postgresql://graph-node:let-me-in@localhost:5432/graph-node", NoTls).unwrap();

        // Lazily create migration for substrate blocktimestamp
        let result = client.execute(
            "CREATE SCHEMA IF NOT EXISTS custom_subgraph_1", &[]
        );
        match result {
            Ok(_) => {
                println!("Substrate - Schema created");
            },
            Err(e) => {
                println!("Substrate - Schema create error: {}", e);
            }
        };

        // Lazily create table for substrate block timestamp
        let result = client.execute(
            "CREATE TABLE IF NOT EXISTS custom_subgraph_1.index (
                        hash bytea NOT NULL,
                        timestamp varchar(50) NOT NULL
                    )", &[]
        );
        match result {
            Ok(_) => {
                println!("Substrate - Table created");
            },
            Err(e) => {
                println!("Substrate - Table create error: {}", e);
            }
        };

        loop {
            let gist_body = json!({
                "jsonrpc": "2.0",
                "method": "chain_getBlockHash",
                "params": [],
                "id": 1
            });
            let request_url = "http://localhost:9933";
            let response = Client::new()
                .post(request_url)
                .json(&gist_body)
                .send().compat().await.unwrap();

            let block_result: BlockResult = response.json().await.unwrap();
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
            println!("Substrate - Syncing 1 block ");

            let result = client.execute(
                "INSERT INTO custom_subgraph_1.index (hash, timestamp) VALUES ($1, $2)",
                &[&block_result.result.as_bytes(), &timestamp.to_string()],
            );

            match result {
                Ok(_) => {
                    println!("Substrate - Data insert success");
                },
                Err(e) => {
                    println!("Substrate - Insert error: {}", e);
                }
            };

            thread::sleep(Duration::from_secs(6));
        }
    }
}