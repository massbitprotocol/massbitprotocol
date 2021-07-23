use diesel::{PgConnection, RunQueryDsl};

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