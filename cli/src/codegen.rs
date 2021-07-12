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

pub fn run(_matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    generate_rust_entity()?;
    generate_plugin()?;
    Ok(())
}

#[derive(Serialize)]
pub struct EntityBinding {
    pub entities: HashMap<String, String>,
}

fn generate_rust_entity() -> Result<(), Box<dyn Error>> {
    let raw_schema = fs::read_to_string("schema.graphql")?;
    let schema = Schema::parse(&raw_schema)?;
    let layout = Layout::new(&schema)?;

    let mut binding = EntityBinding {
        entities: HashMap::new(),
    };
    for (name, model) in layout.models.into_iter() {
        let mut s = String::new();
        model.as_rust_struct(&mut s)?;
        binding.entities.insert(name, s);
    }

    let mut tera = Tera::default();
    tera.add_raw_template("models", include_str!("templates/models.rs.tmpl"))?;
    let data = tera.render("models", &Context::from_serialize(binding)?)?;
    fs::write("src/models.rs", data)?;

    Ok(())
}

#[derive(Serialize)]
pub struct HandlerBinding {
    pub handlers: HashMap<String, String>,
}

fn generate_plugin() -> Result<(), Box<dyn Error>> {
    let f = File::open("project.yaml")?;
    let manifest: serde_yaml::Value = serde_yaml::from_reader(f)?;
    let handlers = manifest["dataSources"][0]["mapping"]["handlers"]
        .as_sequence()
        .unwrap();
    let mut binding = HandlerBinding {
        handlers: HashMap::new(),
    };
    for (_, handler) in handlers.iter().enumerate() {
        let name = handler["handler"].as_str().map(|s| s.to_string()).unwrap();
        let kind = handler["kind"].as_str().map(|s| s.to_string()).unwrap();
        binding.handlers.insert(name.to_snake_case(), kind);
    }

    let mut tera = Tera::default();
    tera.add_raw_template("plugin", include_str!("templates/lib.rs.tmpl"))?;
    let data = tera.render("plugin", &Context::from_serialize(binding)?)?;
    fs::write("src/lib.rs", data)?;

    Ok(())
}
