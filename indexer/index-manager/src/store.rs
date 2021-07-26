use diesel::{PgConnection, RunQueryDsl};
use std::process::Command;

pub fn create_new_indexer_detail_table(connection: &PgConnection, raw_query: &String) {
    let query = diesel::sql_query(raw_query.clone());
    println!("Running: {}", raw_query);
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
            log::info!("[Index Manager Helper] New indexer created");
        }
        Err(e) => {
            log::warn!("[Index Manager Helper] {}", e);
        }
    };
}

pub fn run_migration_cli() {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg("store/postgres/Cargo.toml")
        .output()
        .expect("failed to execute process");

    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success());
}