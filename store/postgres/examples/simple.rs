use graphql::ddlgen;
use std::error::Error;
use std::{env, fs};
use chrono::{DateTime, Utc};
use graphql::command_support::{Namespace, Layout, Catalog};
use graph::prelude::{q, s, Schema, SubgraphDeploymentId, StoreError, ValueType};
use lazy_static::lazy_static;

lazy_static! {
    static ref THINGS_SUBGRAPH_ID: SubgraphDeploymentId = SubgraphDeploymentId::new("subgraphId").unwrap();
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let raw_schema = fs::read_to_string("schema.graphql")?;
    let namespace = String::from("catalog");
    let now: DateTime<Utc> = Utc::now();
    //let out_dir = format!("./migrations/{:?}", now.format("%Y-%m-%d-%H%M%S").to_string());
    let output_dir = String::from("./migrations/") + now.format("%Y%m%d%H%M%S").to_string().as_str();
    let schema = Schema::parse(raw_schema.as_str(), THINGS_SUBGRAPH_ID.clone())?;
    let catalog = Catalog::make_empty(Namespace::new(namespace)?)?;
    match Layout::new(&schema, catalog, false) {
        Ok(layout) => {
            for (_, table) in &layout.tables {
                for column in &table.columns {
                    println!("{:?}", column);
                }
            }
            layout.tables.iter().map(|(name, table)|{
                println!("{}", name);
                println!("{}", table.columns.iter().map(|column|{
                    column.field_type.to_string()
                }).collect::<Vec<String>>().join(";"));
                name
            }).collect::<Vec<&String>>();
            let result = layout.as_ddls()?;
            fs::create_dir_all(output_dir.as_str())?;
            fs::write(format!("{}/up.sql", output_dir.as_str()), result.0.join(";"))?;
            fs::write(format!("{}/hasura_queries.json", output_dir.as_str()), result.1.join(";"))?;
            Ok(())
        },
        Err(_err) => Err(format!("Invalid schema").into())
    }
}