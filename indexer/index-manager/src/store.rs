use diesel::{PgConnection, RunQueryDsl};
use std::process::Command;

pub fn create_new_indexer_detail_table(connection: &PgConnection, raw_query: &String) {
    let query = diesel::sql_query(raw_query.clone());
    log::info!("[Index Manager Store] Creating new indexer by raw_query: {}", raw_query);
    query.execute(connection);
}

pub fn insert_new_indexer(
    connection: &PgConnection,
    id: &String,
    project_config: &serde_yaml::Value,
) {
    let network = project_config["dataSources"][0]["kind"].as_str().unwrap();
    let name = project_config["dataSources"][0]["name"].as_str().unwrap();

    let add_new_indexer = format!(
        "INSERT INTO indexers(id, name, network) VALUES ('{}','{}','{}');",
        id, name, network
    );
    let result = diesel::sql_query(add_new_indexer).execute(connection);
    match result {
        Ok(_) => {
            log::info!("[Index Manager Store] New indexer created");
        }
        Err(e) => {
            log::warn!("[Index Manager Store] {}", e);
        }
    };
}

pub fn plugin_migration(index_name: &String, schema: &String, config: &String) {
    log::debug!("[Index Manager Store] index_name {}", index_name);
    log::debug!("[Index Manager Store] schema {}", schema);
    log::debug!("[Index Manager Store] config {}", config);
    let output = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg("store/postgres/Cargo.toml")
        .arg("--")
        .arg("ddlgen")
        .arg("-h")
        .arg(index_name)
        .arg("-c")
        .arg(config)
        .arg("-s")
        .arg(schema)
        .output()
        .expect("failed to execute plugin migration");

    log::info!("[Index Manager Store] status: {}", output.status);
    log::info!("[Index Manager Store] stdout: {}", String::from_utf8_lossy(&output.stdout));
    assert!(output.status.success());
}
