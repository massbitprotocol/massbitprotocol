use std::error::Error;
use graph::prelude::{Schema, SubgraphDeploymentId};
//use graph_store_postgres::command_support::{Namespace, Catalog, Layout};
use super::{catalog::{Catalog}, primary::Namespace, relational::{Layout} };
use lazy_static::lazy_static;

lazy_static! {
    static ref THINGS_SUBGRAPH_ID: SubgraphDeploymentId = SubgraphDeploymentId::new("subgraphId").unwrap();
}

///
/// Parse input schema to pure pgsql sql for creating tables in database.
/// Input: graphql schema, namespace in database
/// Output: Vector of pure queries
///
pub fn gen_ddls(raw: &str, namespace: &str) -> Result<(Vec<String>, Vec<String>), Box<dyn Error>> {

    let schema = Schema::parse(raw, THINGS_SUBGRAPH_ID.clone())?;
    let mut ns = String::from(namespace);
    if !ns.starts_with("sgd") || ns.len() <= 3 {
        ns.insert_str(0, "sgd");
    }
    let catalog = Catalog::make_empty(Namespace::new(ns)?)?;
    match Layout::new(&schema, catalog, false) {
        Ok(layout) => {
            let result = layout.as_ddls()?;
            Ok(result)
        },
        Err(_err) => Err(format!("Invalid schema").into())
    }
}

pub fn gen_ddl(raw: &str, namespace: &str) -> Result<String, Box<dyn Error>> {

    let schema = Schema::parse(raw, THINGS_SUBGRAPH_ID.clone())?;
    let mut ns = String::from(namespace);
    if !ns.starts_with("sgd") || ns.len() <= 3 {
        ns.insert_str(0, "sgd");
    }
    let catalog = Catalog::make_empty(Namespace::new(ns)?)?;
    match Layout::new(&schema, catalog, false) {
        Ok(layout) => {
            let result = layout.as_ddl()?;
            Ok(result)
        },
        Err(_err) => Err(format!("Invalid schema").into())
    }
}
/*
pub fn parse(raw: &str, namespace: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut result : Vec<String> = Vec::new();
    let schema = Schema::parse(raw, THINGS_SUBGRAPH_ID.clone())?;
    //let namespace = Namespace::new(namespace.to_string());
    //let ns = Namespace::new(namespace.to_string()).unwrap();
    let mut ns = String::from(namespace);
    if !ns.starts_with("sgd") || ns.len() <= 3 {
        ns.insert_str(0, "sgd");
    }
    let catalog = Catalog::make_empty(Namespace::new(ns)?)?;
    match Layout::new(&schema, catalog, false) {
        Ok(layout) => {
            for (name, values) in &layout.enums {
                let mut out = String::new();
                let mut sep = "";
                let name = elements::SqlName::from(name.as_str());
                write!(
                    out,
                    "create type {}.{}\n    as enum (",
                    layout.catalog.namespace,
                    name.quoted()
                )?;
                for value in values.iter() {
                    write!(out, "{}'{}'", sep, value)?;
                    sep = ", "
                }
                writeln!(out, ");")?;
                result.push(out);
            }
            // We sort tables here solely because the unit tests rely on
            // 'create table' statements appearing in a fixed order
            let mut tables = layout.tables.values().collect::<Vec<_>>();
            //tables.sort_by_key(|table| table.position);
            // Output 'create table' statements for all tables
            for table in tables {
                let mut out = String::new();
                table.as_ddl(&mut out, layout)?;
            }
            Ok(result)
        },
        Err(err) => Err(format!("Invalid schema").into())
    }
}
 */