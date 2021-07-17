use clap::{App, Arg};

mod codegen;
mod graphql;

fn main() {
    let matches = App::new("massbit-cli")
        .version("1.0")
        .about("Massbit CLI")
        .subcommand(
            App::new("codegen")
                .about("Generate Rust code & SQL migrations from GraphQL schema")
                .arg(
                    Arg::new("config")
                        .about("project yaml file path")
                        .takes_value(true)
                        .short('c')
                        .long("config"),
                )
                .arg(
                    Arg::new("schema")
                        .about("graphql schema file path")
                        .takes_value(true)
                        .short('s'),
                )
                .arg(
                    Arg::new("output")
                        .about("codegen output directory")
                        .takes_value(true)
                        .short('o'),
                ),
        )
        .get_matches();

    if let Some(ref matches) = matches.subcommand_matches("codegen") {
        codegen::run(matches).unwrap();
    }
}
