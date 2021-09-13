/**
*** Objective of this file is to write to databases that are related to indexer
*** like: indexer list, indexer detail, ...
*** Also, there's a helper function to call to DDL Gen to migrate data
**/
// Generic dependencies
use diesel::{Connection, PgConnection, RunQueryDsl};
use lazy_static::lazy_static;
use postgres::{Client, NoTls};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use strum::AsStaticRef;

// Massbit dependencies
use crate::ipfs::read_config_file;
use crate::type_index::{IndexConfig, IndexStatus, IndexStore, Indexer};

lazy_static! {
    static ref INDEXER_MIGRATION_FILE: String =
        String::from("./index-manager/migration/indexers.sql");
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
}

impl IndexStore {
    pub fn create_indexers_table_if_not_exists() {
        let connection = PgConnection::establish(&DATABASE_CONNECTION_STRING).expect(&format!(
            "Error connecting to {}",
            *DATABASE_CONNECTION_STRING
        ));

        let mut query = String::new();
        let mut f = File::open(&*INDEXER_MIGRATION_FILE).expect("Unable to open file");
        f.read_to_string(&mut query).expect("Unable to read string"); // Get raw query
        let result = diesel::sql_query(query).execute(&connection);
        match result {
            Ok(_) => {}
            Err(e) => {
                log::warn!("[Index Manager Store] {}", e);
            }
        };
    }

    // Create a new indexer so we can keep track of it's status
    pub fn insert_new_indexer(index_config: &IndexConfig) {
        IndexStore::create_indexers_table_if_not_exists();
        let connection = PgConnection::establish(&DATABASE_CONNECTION_STRING).expect(&format!(
            "Error connecting to {}",
            *DATABASE_CONNECTION_STRING
        ));

        let id = &index_config.identifier.name_with_hash;
        let config_value = read_config_file(&index_config.config);

        let network = config_value["dataSources"][0]["kind"].as_str().unwrap();
        let name = config_value["dataSources"][0]["name"].as_str().unwrap();

        let add_new_indexer = format!(
            "INSERT INTO indexers(id, name, network, index_status, hash) VALUES ('{}','{}','{}', '{}', '{}');",
            id,
            name,
            network,
            IndexStatus::Synced.as_static().to_lowercase(),
            index_config.identifier.hash,
        );
        let result = diesel::sql_query(add_new_indexer).execute(&connection);
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

    pub fn get_indexer_list() -> Vec<Indexer> {
        IndexStore::create_indexers_table_if_not_exists();

        // User Postgre create for easy query. Should later switch to use Diesel
        let mut client =
            Client::connect(DATABASE_CONNECTION_STRING.clone().as_str(), NoTls).unwrap();
        let mut indexers: Vec<Indexer> = Vec::new();

        for row in &client
            .query("SELECT id, network, name, hash FROM indexers", &[])
            .unwrap()
        {
            let indexer = Indexer {
                id: row.get(0),
                network: row.get(1),
                name: row.get(2),
                hash: row.get(3),
            };
            indexers.push(indexer);
        }
        indexers
    }
}
