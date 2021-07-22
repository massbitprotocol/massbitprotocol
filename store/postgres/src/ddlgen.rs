use std::error::Error;
use std::sync::Arc;
use std::convert::{From, TryFrom, TryInto};
use std::fs;
use std::str::FromStr;
use graph::prelude::{q, s, Schema, SubgraphDeploymentId, StoreError, ValueType};
use graph::data::{
    graphql::ext::{DocumentExt, ObjectTypeExt},
    subgraph::schema::MetadataType,
};
use graph::data::schema::{FulltextConfig, FulltextDefinition, SCHEMA_TYPE_NAME};

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
//use graph_store_postgres::command_support::{Namespace, Catalog, Layout};
//use store_postgres::{catalog::{Catalog}, primary::Namespace, relational::{Layout} };
//use super::command_support::{Layout, Namespace, Catalog};
use lazy_static::lazy_static;
use clap::ArgMatches;
use serde_yaml::{Value, Mapping};
use crate::relational::{Layout, SqlName, PRIMARY_KEY_COLUMN};
use crate::primary::Namespace;
use crate::catalog::Catalog;
lazy_static! {
    static ref THINGS_SUBGRAPH_ID: SubgraphDeploymentId = SubgraphDeploymentId::new("subgraphId").unwrap();
}
/*
/// The SQL type to use for GraphQL ID properties. We support
/// strings and byte arrays
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum IdType {
    String,
    Bytes,
}
impl TryFrom<&s::ObjectType> for IdType {
    type Error = StoreError;

    fn try_from(obj_type: &s::ObjectType) -> Result<Self, Self::Error> {
        let pk = obj_type
            .field(&PRIMARY_KEY_COLUMN.to_owned())
            .expect("Each ObjectType has an `id` field");
        Self::try_from(&pk.field_type)
    }
}

impl TryFrom<&s::Type> for IdType {
    type Error = StoreError;

    fn try_from(field_type: &s::Type) -> Result<Self, Self::Error> {
        let name = named_type(field_type);

        match ValueType::from_str(name)? {
            ValueType::String => Ok(IdType::String),
            ValueType::Bytes => Ok(IdType::Bytes),
            _ => Err(anyhow!(
                "The `id` field has type `{}` but only `String`, `Bytes`, and `ID` are allowed",
                &name
            )
                .into()),
        }
    }
}

type IdTypeMap = HashMap<String, IdType>;
type EnumMap = BTreeMap<String, Arc<BTreeSet<String>>>;
*/
pub fn run(matches: &ArgMatches, dbconfig : &Value) -> Result<(), Box<dyn Error>> {
    let schema_path = matches.value_of("schema").unwrap_or("schema.graphql");
    let output = matches.value_of("output").unwrap_or("migrations");
    let def_cat = Value::String(String::from("subgraph"));
    let catalog = dbconfig.get("catalog").unwrap_or(&def_cat).as_str().unwrap();
    let raw_schema = fs::read_to_string(schema_path)?;
    generate_ddl(raw_schema.as_str(), catalog,output)
}

///
/// Parse input schema to pure pgsql sql for creating tables in database.
/// Input: graphql schema, namespace in database
/// Output: Vector of pure queries
///

pub fn generate_ddl(raw: &str, catalog: &str, output_dir: &str) -> Result<(), Box<dyn Error>> {
    //let mut ddls : Vec<String> = Vec::new();
    //let mut table_names : Vec<String> = Vec::new();
    let schema = Schema::parse(raw, THINGS_SUBGRAPH_ID.clone())?;
    let catalog = Catalog::make_empty(Namespace::new(String::from(catalog))?)?;
    match Layout::new(&schema, catalog, false) {
        Ok(layout) => {
            let result = layout.as_ddls()?;
            fs::write(format!("{}/up.sql", output_dir), result.0.join(";"))?;
            fs::write(format!("{}/tables.txt", output_dir), result.1.join(";"))?;
            Ok(())
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