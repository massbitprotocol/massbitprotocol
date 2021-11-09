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
                let data = schema.gen_instruction();
                self.write_to_file("generated/instruction.rs", &data);
                let data = schema.gen_graphql_schema();
                self.write_to_file("schema.graphql", &data);
                let data = schema.gen_handler();
                self.write_to_file("generated/handler.rs", &data);
                let data = schema.gen_models();
                self.write_to_file("models.rs", &data);
            }
        }
    }
    pub fn write_to_file(&self, file_name: &str, content: &String) {
        let path = format!("{}/{}", self.output_dir, file_name);
        match fs::write(path.as_str(), content) {
            Ok(_) => {
                log::info!("Write content to file {} successfully", path);
            }
            Err(err) => {
                log::info!("Write content to file {} fail. {:?}", path, &err);
            }
        };
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
