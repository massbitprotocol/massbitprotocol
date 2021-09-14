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
                .arg(Arg::with_name("config").takes_value(true).short("c"))
                .arg(Arg::with_name("schema").takes_value(true).short("s"))
                .arg(Arg::with_name("output").takes_value(true).short("o"))
                .arg(Arg::with_name("mapping_gen").takes_value(false).short("m")),
        )
        .get_matches();

    if let Some(ref matches) = matches.subcommand_matches("codegen") {
        codegen::run(matches).unwrap();
    }
}
