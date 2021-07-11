use clap::App;

mod codegen;
mod graphql;

fn main() {
    let matches = App::new("massbit-cli")
        .version("1.0")
        .about("Massbit CLI")
        .subcommand(
            App::new("codegen").about("Generate Rust code & SQL migrations from GraphQL schema"),
        )
        .get_matches();

    match matches.subcommand_name() {
        Some("codegen") => codegen::run(&matches).unwrap(),
        _ => println!("Some other subcommand was used"),
    }
}
