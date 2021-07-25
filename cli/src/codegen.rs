use crate::graphql::relational::Layout;
use crate::graphql::schema::Schema;

use clap::ArgMatches;
use inflector::cases::snakecase::to_snake_case;
use inflector::Inflector;
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use tera::{Context, Tera};

pub fn run(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let schema_path = matches.value_of("schema").unwrap_or("schema.graphql");
    let output = matches.value_of("output").unwrap_or("src");
    generate_rust_entity(schema_path, output)?;

    let config_path = matches.value_of("config").unwrap_or("project.yaml");
    generate_plugin(config_path, output)?;
    Ok(())
}

#[derive(Serialize)]
pub struct EntityBinding {
    pub entities: HashMap<String, String>,
}

fn generate_rust_entity(schema_path: &str, output: &str) -> Result<(), Box<dyn Error>> {
    let raw_schema = fs::read_to_string(schema_path)?;
    let schema = Schema::parse(&raw_schema)?;
    let layout = Layout::new(&schema)?;

    let mut binding = EntityBinding {
        entities: HashMap::new(),
    };
    for (name, model) in layout.models.into_iter() {
        let mut s = String::new();
        model.as_rust(&mut s)?;
        binding.entities.insert(name, s);
    }

    let mut tera = Tera::default();
    tera.add_raw_template("model", include_str!("templates/model.rs.tmpl"))?;
    let data = tera.render("model", &Context::from_serialize(binding)?)?;
    fs::write(format!("{}/model.rs", output), data)?;

    Ok(())
}

#[derive(Serialize, Default)]
pub struct HandlerBinding {
    pub handlers: Vec<Handler>,
}

#[derive(Serialize, Default)]
pub struct Handler {
    pub name: String,
    pub kind: String,
}

fn generate_plugin(config_path: &str, output: &str) -> Result<(), Box<dyn Error>> {
    let f = File::open(config_path)?;
    let manifest: serde_yaml::Value = serde_yaml::from_reader(f)?;
    let mut binding = HandlerBinding::default();
    let data_sources = manifest["dataSources"].as_sequence().unwrap();
    for (_, ds) in data_sources.iter().enumerate() {
        let handlers = ds["mapping"]["handlers"].as_sequence().unwrap();
        for (_, handler) in handlers.iter().enumerate() {
            let name = handler["handler"].as_str().map(|s| s.to_string()).unwrap();
            let kind = handler["kind"].as_str().map(|s| s.to_string()).unwrap();
            binding.handlers.push(Handler {
                name: name.to_snake_case(),
                kind,
            })
        }
    }

    let mut tera = Tera::default();
    tera.add_raw_template("lib", include_str!("templates/lib.rs.tmpl"))?;
    let data = tera.render("lib", &Context::from_serialize(&binding)?)?;
    fs::write(format!("{}/lib.rs", output), data)?;

    tera.add_raw_template("mapping", include_str!("templates/mapping.rs.tmpl"))?;
    let data = tera.render("mapping", &Context::from_serialize(&binding)?)?;
    fs::write(format!("{}/mapping.rs", output), data)?;

    Ok(())
}
