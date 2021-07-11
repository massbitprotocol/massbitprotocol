use crate::graphql::relational::Layout;
use crate::graphql::schema::Schema;

use clap::ArgMatches;
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use tera::{Context, Tera};

pub fn run(_matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    generate_rust_entity()?;
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
    tera.add_raw_template("models", include_str!("templates/models.rs"))?;
    let data = tera.render("models", &Context::from_serialize(binding)?)?;
    fs::write("src/models.rs", data)?;

    Ok(())
}
