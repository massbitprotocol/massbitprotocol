pub mod gen_lib;
pub mod gen_mapping;
pub mod graphql;
pub mod handler;
pub mod helper;
pub mod instruction;
pub mod model;

use crate::schema::Schema;
use gen_lib::gen_lib;
use gen_mapping::gen_mapping;
use serde::ser::Serialize;
use std::fs;
use std::fs::DirEntry;
use std::io::Error;
use std::{
    io,
    path::{Path, PathBuf},
};

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
                //Instruction
                let data = schema.gen_instruction();
                let path = format!("{}/{}", self.output_dir, "generated/instruction.rs");
                self.write_to_file(path.as_str(), &data, true);
                //Instruction handler
                let data = schema.gen_handler();
                let path = format!("{}/{}", self.output_dir, "generated/handler.rs");
                self.write_to_file(path.as_str(), &data, true);
                //Models
                let data = schema.gen_models();
                let path = format!("{}/{}", self.output_dir, "models.rs");
                self.write_to_file(path.as_str(), &data, true);
                //libs
                let lib_code = gen_lib();
                let path = format!("{}/{}", self.output_dir, "lib.rs");
                self.write_to_file(path.as_str(), &lib_code, true);
                //Mapping
                let mapping_code = gen_mapping();
                let path = format!("{}/{}", self.output_dir, "mapping.rs");
                self.write_to_file(path.as_str(), &mapping_code, true);
                //Schema graphql
                let data = schema.gen_graphql_schema();
                let path = format!("{}/{}", self.output_dir, "schema.graphql");
                self.write_to_file(path.as_str(), &data, false);
            }
        }
    }

    //pub fn generate_to_file<P: ?Sized + AsRef<Path>>(&self, output_file: &'b P) -> io::Result<()> {

    pub fn write_to_file<P: ?Sized + AsRef<Path>>(
        &self,
        output_path: &P,
        content: &String,
        apply_format: bool,
    ) -> io::Result<()> {
        match fs::write(output_path, content) {
            Ok(_) => {
                if apply_format {
                    use std::process::Command;
                    Command::new("rustfmt")
                        .arg(output_path.as_ref().as_os_str())
                        .output();
                }
                log::info!(
                    "Write content to file {:?} successfully",
                    &output_path.as_ref().as_os_str()
                );
                Ok(())
            }
            e @ Err(_) => {
                log::info!(
                    "Write content to file {:?} fail. {:?}",
                    &output_path.as_ref().as_os_str(),
                    &e
                );
                e
            }
        }
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
