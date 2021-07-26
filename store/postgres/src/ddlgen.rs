use std::error::Error;
use std::convert::From;
use std::{env, fs};
use graph::prelude::{Schema, SubgraphDeploymentId};

//use graph_store_postgres::command_support::{Namespace, Catalog, Layout};
//use store_postgres::{catalog::{Catalog}, primary::Namespace, relational::{Layout} };
//use super::command_support::{Layout, Namespace, Catalog};
use lazy_static::lazy_static;
use clap::ArgMatches;
use serde_yaml::{Value, Mapping};
use crate::relational::Layout;
use crate::primary::Namespace;
use crate::catalog::Catalog;
//use crate::metrics_registry::*;
use chrono::{DateTime, Utc};
use diesel::{PgConnection, Connection};
use diesel_migrations;
use std::path::{PathBuf};
use serde_json;
use std::fs::File;


lazy_static! {
    static ref THINGS_SUBGRAPH_ID: SubgraphDeploymentId = SubgraphDeploymentId::new("subgraphId").unwrap();
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
}
//const MIGRATION_PATH: &str = r#"./migrations"#;

pub fn run(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config_path = matches.value_of("config").unwrap_or("project.yaml");
    let fd = File::open(config_path).unwrap();
    let manifest: serde_yaml::Value = serde_yaml::from_reader(fd).unwrap();
    let def_map = Value::Mapping(Mapping::new());
    let dbconfig = manifest.get("database").unwrap_or(&def_map);
    //Default db catalog - currently not support custom catalog
    let def_catalog = Value::String(String::from("graph-node"));
    let catalog = dbconfig.get("catalog").unwrap_or(&def_catalog).as_str().unwrap();
    //input schema path
    let schema_path = matches.value_of("schema").unwrap_or("schema.graphql");
    let session = matches.value_of("hash").unwrap_or("");
    let output = matches.value_of("output").unwrap_or("./migrations");

    let raw_schema = fs::read_to_string(schema_path)?;
    let now: String = Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
    //include session hash in output dir
    let out_dir = format!("{}/{}_{}", output, now.as_str(), session);

    match generate_ddl(raw_schema.as_str(), catalog, out_dir.as_str()) {
        Ok(_) => {
            let url = format!("{}/{}",DATABASE_CONNECTION_STRING.as_str(), catalog);
            let path = PathBuf::from(out_dir.as_str());
            run_migrations(path, url.as_str());
            Ok(())
        }
        Err(err) => Err(err)
    }
}
///
/// Run diesel migrations
///
fn run_migrations(path: PathBuf, db_url : &str) -> Result<(), Box<dyn Error>>{
    log::info!("Migration path: {:?}", &path);
    match diesel_migrations::migration_from(path) {
        Ok(migration) => {
            let list_migrations = vec![migration];
            let connection = PgConnection::establish(&db_url).expect(&format!(
                "Error connecting to {}",
                *DATABASE_CONNECTION_STRING
            ));
            diesel_migrations::run_migrations(&connection, list_migrations, &mut std::io::stdout());
        }
        Err(err) => {
            println!("{:?}", err);
        }
    };
    Ok(())
}
///
/// Parse input schema to pure pgsql sql for creating tables in database.
/// Input: graphql schema, namespace in database
/// Output: 3 files on disk: up.sql, down.sql, hasura_queries.json
///

pub fn generate_ddl(raw: &str, catalog: &str, output_dir: &str) -> Result<(), Box<dyn Error>> {
    //let mut ddls : Vec<String> = Vec::new();
    //let mut table_names : Vec<String> = Vec::new();
    let schema = Schema::parse(raw, THINGS_SUBGRAPH_ID.clone())?;
    //println!("{}",schema.document.to_string());
    let catalog = Catalog::make_empty(Namespace::new(String::from(catalog))?)?;
    match Layout::new(&schema, catalog, false) {
        Ok(layout) => {
            let result = layout.gen_migration()?;
            let mut queries : Vec<String> = Vec::new();
            //Tobe improved
            layout.tables.iter().for_each(|(name,_table)| {
                let query = serde_json::json!({
                    "type": "track_table",
                    "args": {
                        "schema": "public",
                        "name": name
                    },
                });
                match serde_json::to_string(&query) {
                    Ok(val) => {
                        queries.push(val);
                    }
                    Err(_) => {}
                }
            });

            fs::create_dir_all(output_dir)?;
            fs::write(format!("{}/up.sql", output_dir), result.0);
            fs::write(format!("{}/down.sql", output_dir),result.1);
            fs::write(format!("{}/hasura_queries.json", output_dir), format!("[\n\t{}\n]", queries.join(",\n\t")));
            Ok(())
        },
        Err(_err) => {
            println!("Error");
            Err(format!("Invalid schema").into())
        }
    }
}
