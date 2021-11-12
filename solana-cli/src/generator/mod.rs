pub mod graphql;
pub mod handler;
pub mod helper;
pub mod indexer_lib;
pub mod indexer_mapping;
pub mod indexer_mod;
pub mod indexer_setting;
pub mod instruction;
//pub mod model;

use crate::generator::indexer_mod::INDEXER_MOD;
use crate::schema::Schema;
use handlebars::Handlebars;
use indexer_lib::INDEXER_LIB;
use indexer_mapping::INDEXER_MAPPING;
use indexer_setting::*;
use minifier::json::minify;
use serde::ser::Serialize;
use serde_json::Value;
use serde_json::{json, to_string};
use std::collections::BTreeMap;
use std::fs;
use std::{io, path::Path};

#[derive(Debug)]
#[must_use]
pub struct Generator<'a> {
    pub structure_path: &'a str,
    pub config_path: &'a str,
    /// The output dir
    pub output_dir: &'a str,
    pub schema: Option<Schema>,
    pub config: Option<Value>,
    pub definitions: BTreeMap<String, &'a Schema>,
}

impl<'a> Generator<'a> {
    pub fn builder() -> GeneratorBuilder<'a> {
        GeneratorBuilder::default()
    }

    pub fn generate(&self) -> Result<(), io::Error> {
        let ref_schema = self.schema.as_ref();
        if let Some(schema) = ref_schema {
            //Instruction
            let data = schema.gen_instruction();
            self.write_to_file(
                &format!("{}/{}", self.output_dir, "src/generated/instruction.rs"),
                &data,
                true,
            )?;
            //Instruction handler
            let data = self.gennerate_handler();
            self.write_to_file(
                &format!("{}/{}", self.output_dir, "src/generated/handler.rs"),
                &data,
                true,
            )?;
            //Models
            // let data = schema.gen_models();
            // self.write_to_file(
            //     &format!("{}/{}", self.output_dir, "src/models.rs"),
            //     &data,
            //     true,
            // )?;
            //libs
            self.write_to_file(
                &format!("{}/{}", self.output_dir, "src/lib.rs"),
                &format!("{}", INDEXER_LIB),
                true,
            )?;
            //Mapping
            self.write_to_file(
                &format!("{}/{}", self.output_dir, "src/mapping.rs"),
                &format!("{}", INDEXER_MAPPING),
                true,
            )?;
            //mod.rs
            self.write_to_file(
                &format!("{}/{}", self.output_dir, "src/generated/mod.rs"),
                &format!("{}", INDEXER_MOD),
                true,
            )?;
            //subgraph.yaml
            let config = self.config.clone().unwrap();
            let name = &config["name"].as_str().unwrap_or_default();
            let contract_address = &config["contract_address"].as_str().unwrap_or_default();
            let start_block = &config["start_block"].as_i64().unwrap_or_default();

            println!("name: {}", name);
            self.write_to_file(
                &format!("{}/{}", self.output_dir, "src/subgraph.yaml"),
                &Handlebars::new()
                    .render_template(
                        INDEXER_YAML,
                        &json!({
                            "name": name,
                            "address": contract_address,
                            "start_block": start_block
                        }),
                    )
                    .unwrap(),
                true,
            )?;
            //Schema graphql
            let data = schema.gen_graphql_schema();
            self.write_to_file(
                &format!("{}/{}", self.output_dir, "src/schema.graphql"),
                &data,
                false,
            )?;
            //Cargo toml
            self.write_to_file(
                &format!("{}/{}", self.output_dir, "Cargo.toml"),
                &format!("{}", CARGO_TOML),
                false,
            )?;
        };
        Ok(())
    }
    //pub fn generate_to_file<P: ?Sized + AsRef<Path>>(&self, output_file: &'b P) -> io::Result<()> {

    // pub fn write_to_file<P: ?Sized + AsRef<Path>>(
    //     &self,
    //     output_path: &P,
    //     content: &String,
    //     apply_format: bool,
    // ) -> io::Result<()> {
    pub fn write_to_file(
        &self,
        output_path: &String,
        content: &String,
        apply_format: bool,
    ) -> io::Result<()> {
        // Check and create parent directory
        let path = std::path::Path::new(&output_path);
        let prefix = path.parent().unwrap();
        std::fs::create_dir_all(prefix).unwrap();

        // Write the file
        match fs::write(output_path, content) {
            Ok(_) => {
                if apply_format {
                    use std::process::Command;
                    Command::new("rustfmt").arg(output_path).output();
                }
                log::info!("Write content to file {:?} successfully", &output_path);
                Ok(())
            }
            e @ Err(_) => {
                log::info!("Write content to file {:?} fail. {:?}", &output_path, &e);
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
                config_path: "",
                output_dir: "",
                schema: None,
                config: None,
                definitions: BTreeMap::default(),
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
    pub fn with_config_path(mut self, path: &'a str) -> Self {
        self.inner.config_path = path;
        let json = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("Unable to read `{}`: {}", path, err));
        let config: Value = serde_json::from_str(&minify(&json))
            .unwrap_or_else(|err| panic!("Cannot parse `{}` as JSON: {}", path, err));
        //println!("config: {:?}", &config);
        self.inner.config = Some(config);
        self
    }
    pub fn with_output_dir(mut self, output_dir: &'a str) -> Self {
        self.inner.output_dir = output_dir;
        self
    }
    fn collect_definitions(&mut self, schema: &'a Schema) {
        schema.definitions.iter().for_each(|(name, schema)| {
            self.inner.definitions.insert(name.clone(), schema);
            self.collect_definitions(schema);
        });
    }
    pub fn build(mut self) -> Generator<'a> {
        self.inner
    }
}
