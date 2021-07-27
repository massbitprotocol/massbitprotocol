use reqwest::Client;
use tokio_compat_02::FutureExt;
use serde_json::{json, Value};
use lazy_static::lazy_static;
use std::{env};

// Massbit dependencies
use crate::hasura_helper::{assert_no_duplicated_index, get_hasura_payload_folder, get_hasura_payload};

lazy_static! {
    static ref HASURA_URL: String = env::var("HASURA_URL").unwrap_or(String::from("http://localhost:8080/v1/query"));
    static ref COMPONENT_NAME: String = String::from("[Index Manger Hasura]");
}

pub async fn track_hasura_table(table_name: &String) {
    let body = json!({
        "type": "track_table",
        "args": {
            "schema": "public",
            "name": table_name.to_lowercase(),
        }
    });
    Client::new()
        .post(&*HASURA_URL)
        .json(&body)
        .send()
        .compat()
        .await
        .unwrap();
}

// The payload of plugin is handled by ddl gen plugin
pub async fn track_hasura_with_ddl_gen_plugin(index_name: &String) {
    log::info!("{} Running plugin hasura", &*COMPONENT_NAME);
    assert_no_duplicated_index(&index_name);
    let folder = get_hasura_payload_folder(&index_name);
    let payload = get_hasura_payload(&folder);
    let v: Value = serde_json::from_str(&payload).unwrap();
    Client::new()
        .post(&*HASURA_URL)
        .json(&v)
        .send()
        .compat()
        .await
        .unwrap();
}
