use clap::{App, Arg, SubCommand};
use graphql::ddlgen;
use std::error::Error;
use serde_yaml::{Value, Mapping};
use std::fs::File;

fn main() -> Result<(), Box<dyn Error>>{
    let matches = App::new("massbit-cli")
        .version("1.0")
        .about("Massbit CLI")
        .subcommand(
            SubCommand::with_name("ddlgen")
                .about("Generate SQL migrations from GraphQL schema")
                .arg(
                    Arg::with_name("config")
                        .help("Project yaml file path")
                        .takes_value(true)
                        .short("c")
                        .long("config"),
                )
                .arg(
                    Arg::with_name("schema")
                        .help("Graphql schema file path")
                        .takes_value(true)
                        .short("s"),
                )
                .arg(
                    Arg::with_name("output")
                        .help("codegen output directory")
                        .takes_value(true)
                        .short("o"),
                ),
        )
        .get_matches();
    let config_path = matches.value_of("config").unwrap_or("project.yaml");
    let fd = File::open(config_path).unwrap();
    let manifest: serde_yaml::Value = serde_yaml::from_reader(fd).unwrap();
    if let Some(ref matches) = matches.subcommand_matches("ddlgen")  {
        let def_map = Value::Mapping(Mapping::new());
        let dbconfig = manifest.get("database").unwrap_or(&def_map);
        ddlgen::run(matches, dbconfig);
    }
    Ok(())
}
