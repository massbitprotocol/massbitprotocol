use crate::graphql::relational::Layout;
use crate::graphql::schema::Schema;
use crate::utils::ensure;

use clap::ArgMatches;
use std::error::Error;
use std::fs;
use tera::{Context, Tera};

pub fn execute(_matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let raw_schema = ensure(
        fs::read_to_string("schema.graphql"),
        "Can not read schema file",
    );

    let schema = Schema::parse(&raw_schema)?;
    let layout = Layout::new(&schema)?;

    let model = layout.models.get("User").unwrap();
    let mut tera = Tera::default();
    tera.add_raw_template("models", include_str!("templates/models.rs"))?;
    let data = tera.render("models", &Context::from_serialize(model)?)?;
    fs::write("models.rs", data)?;
    Ok(())
}
