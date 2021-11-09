pub mod graphql;
pub mod handler;
pub mod instruction;
pub mod model;

use crate::schema::Schema;
use serde::ser::Serialize;
use std::fs::DirEntry;
use std::path::Path;
use std::{fs, io};
use tera::{Context, Tera};

#[derive(Debug)]
#[must_use]
pub struct Generator<'a> {
    pub structure_path: &'a str,
    /// The output dir
    pub output_dir: &'a str,
    pub schema: Option<Schema>,
}

impl<'a> Generator<'a> {
    pub fn builder() -> GeneratorBuilder<'a> {
        GeneratorBuilder::default()
    }
    pub fn generate(&self) {
        match &self.schema {
            None => {}
            Some(schema) => {
                schema.gen_instruction();
                let path = format!("{}/{}", self.output_dir, "instruction.rs");
                match fs::write(path.as_str(), &data) {
                    Ok(_) => {
                        log::info!("Generate {} successfully", &path);
                    }
                    Err(err) => {
                        log::info!("Generate {} fail. {:?}", &path, &err);
                    }
                };
            }
        }
        self.generate_handler("handler", "handler.rs");
        self.generate_to_file("instruction", "instruction.rs");
        self.generate_to_file("model", "model.rs");
        self.generate_to_file("schema", "schema.graphql");
    }
    pub fn generate_to_file(&self, name: &str, output: &str) -> Result<(), anyhow::Error> {
        if self.context.is_some() {
            log::info!("Generate template {}", name);
            println!("Generate template {}", name);
            //println!("{:?}", &self.tera);
            match self.tera.render(name, self.context.as_ref().unwrap()) {
                Ok(data) => {
                    let path = format!("{}/{}", self.output_dir, output);
                    match fs::write(path.as_str(), &data) {
                        Ok(_) => {
                            log::info!("Generate {} successfully", &path);
                        }
                        Err(err) => {
                            log::info!("Generate {} fail. {:?}", &path, &err);
                        }
                    };
                }
                Err(err) => {
                    log::error!("{:?}", &err);
                    println!("{:?}", &err);
                }
            }
        }
        Ok(())
    }
}

pub struct GeneratorBuilder<'a> {
    inner: Generator<'a>,
}

impl<'a> Default for GeneratorBuilder<'a> {
    fn default() -> Self {
        Self {
            inner: Generator {
                structure_path: "",
                output_dir: "",
                schema: None,
            },
        }
    }
}

impl<'a> GeneratorBuilder<'a> {
    pub fn with_structure_path(mut self, path: &'a str) -> Self {
        self.inner.structure_path = path;
        let json = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("Unable to read `{}`: {}", path, err));

        let schema: Schema = serde_json::from_str(&json)
            .unwrap_or_else(|err| panic!("Cannot parse `{}` as JSON: {}", path, err));
        //println!("{:?}", &schema.definitions);
        self.inner.schema = Some(schema);
        self
    }
    pub fn with_output_dir(mut self, output_dir: &'a str) -> Self {
        self.inner.output_dir = output_dir;
        self
    }
    pub fn build(mut self) -> Generator<'a> {
        self.inner
    }
}
