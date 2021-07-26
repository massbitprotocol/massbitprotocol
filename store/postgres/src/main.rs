extern crate exitcode;
use clap::{App, Arg, SubCommand};
use graphql::ddlgen;
use std::error::Error;
use serde_yaml::{Value, Mapping};
use std::fs::File;
use std::process;
fn main() {
    env_logger::init();
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
                )
                .arg(
                    Arg::with_name("hash")
                        .help("Session id or current running")
                        .takes_value(true)
                        .short("h"),
                ),

        )
        .get_matches();
    if let Some(ref matches) = matches.subcommand_matches("ddlgen")  {
        ddlgen::run(matches);
    }
    process::exit(exitcode::OK);
}
