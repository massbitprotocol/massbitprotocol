/**
*** Objective of this file is to write to databases that are related to indexer
*** like: indexer list, indexer detail, ...
*** Also, there's a helper function to call to DDL Gen to migrate data
**/
// Generic dependencies
use diesel::{PgConnection, RunQueryDsl};
use lazy_static::lazy_static;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use strum::AsStaticRef;

// Massbit dependencies
use crate::types::{IndexStatus, IndexStore};

lazy_static! {
    static ref INDEXER_MIGRATION_FILE: String =
        String::from("./index-manager/migration/indexers.sql");
}

impl IndexStore {
    // Create indexers table so we can keep track of the indexers status.
    pub fn create_indexers_table_if_not_exists(connection: &PgConnection) {
        let mut query = String::new();
        let mut f = File::open(&*INDEXER_MIGRATION_FILE).expect("Unable to open file");
        f.read_to_string(&mut query).expect("Unable to read string"); // Get raw query
        let result = diesel::sql_query(query).execute(connection);
        match result {
            Ok(_) => {}
            Err(e) => {
                log::warn!("[Index Manager Store] {}", e);
            }
        };
    }

    // Create a new indexer so we can keep track of it's status
    pub fn insert_new_indexer(
        connection: &PgConnection,
        id: &String,
        project_config: &serde_yaml::Value,
    ) {
        let network = project_config["dataSources"][0]["kind"].as_str().unwrap();
        let name = project_config["dataSources"][0]["name"].as_str().unwrap();

        let add_new_indexer = format!(
            "INSERT INTO indexers(id, name, network, index_status) VALUES ('{}','{}','{}', '{}');",
            id,
            name,
            network,
            IndexStatus::Synced.as_static().to_lowercase()
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

    // Allow user to run raw query
    pub fn run_raw_query(connection: &PgConnection, raw_query: &String) {
        let query = diesel::sql_query(raw_query.clone());
        log::info!("[Index Manager Store] Running raw_query: {}", raw_query);
        query.execute(connection);
    }

    pub fn migrate_with_ddl_gen_plugin(index_name: &String, schema: &PathBuf, config: &PathBuf) {
        log::debug!("[Index Manager Store] Index name: {}", index_name);
        log::debug!(
            "[Index Manager Store] Index schema: {:?}",
            schema.clone().into_os_string().into_string()
        );
        log::debug!(
            "[Index Manager Store] Index config: {:?}",
            config.clone().into_os_string().into_string()
        );
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

        log::info!(
            "[Index Manager Store] Plugin migration status: {}",
            output.status
        );
        log::info!(
            "[Index Manager Store] Plugin migration stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        log::error!(
            "[Index Manager Store] Plugin migration stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success());
    }
}
