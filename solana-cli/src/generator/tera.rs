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
    context: Option<Context>,
    tera: Tera,
}

impl<'a> Generator<'a> {
    pub fn builder() -> GeneratorBuilder<'a> {
        GeneratorBuilder::default()
    }
    pub fn generate(&self) {
        self.generate_to_file("handler", "handler.rs");
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
                    let path = format!(
                        "{}",
                        Path::new(self.output_dir)
                            .join(output)
                            .to_str()
                            .unwrap_or_default()
                    );
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
                context: None,
                tera: Tera::default(),
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
        self.inner.context = Context::from_serialize(schema).ok();
        self
    }
    pub fn with_output_dir(mut self, output_dir: &'a str) -> Self {
        self.inner.output_dir = output_dir;
        self
    }
    pub fn build(mut self) -> Generator<'a> {
        match self
            .inner
            .tera
            .add_raw_template("handler", include_str!("templates/handler.rs.tpl"))
        {
            Ok(val) => {
                println!("Add template handler successfully");
            }
            Err(err) => {
                println!("Error while parse template handler");
            }
        };
        match self
            .inner
            .tera
            .add_raw_template("instruction", include_str!("templates/instruction.rs.tpl"))
        {
            Ok(val) => {
                println!("Add template instruction successfully");
            }
            Err(err) => {
                println!("Error while parse template instruction");
            }
        };
        match self
            .inner
            .tera
            .add_raw_template("model", include_str!("templates/model.rs.tpl"))
        {
            Ok(val) => {
                println!("Add template model successfully");
            }
            Err(err) => {
                println!("Error while parse template model");
            }
        };
        match self
            .inner
            .tera
            .add_raw_template("schema", include_str!("templates/schema.graphql.tpl"))
        {
            Ok(val) => {
                println!("Add template schema successfully");
            }
            Err(err) => {
                println!("Error while parse template schema");
            }
        }
        self.inner
    }
}
