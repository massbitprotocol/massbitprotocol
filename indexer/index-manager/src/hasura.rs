use reqwest::Client;
use tokio_compat_02::FutureExt;
use serde_json::json;
use lazy_static::lazy_static;
use std::{env};

lazy_static! {
    static ref HASURA_URL: String =
        env::var("HASURA_URL").unwrap_or(String::from("http://localhost:8080/v1/query"));
}

pub async fn track_hasura_table(table_name: &String) {
    let gist_body = json!({
        "type": "track_table",
        "args": {
            "schema": "public",
            "name": table_name.to_lowercase(),
        }
    });
    Client::new()
        .post(&*HASURA_URL)
        .json(&gist_body)
        .send()
        .compat()
        .await
        .unwrap();
}