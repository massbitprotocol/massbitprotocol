use clap::App;

mod graphql;
mod schema;
mod utils;

fn main() {
    let matches = App::new("massbit-cli")
        .version("1.0")
        .about("Massbit CLI")
        .subcommand(App::new("schema").about("Generate Rust entity from GraphQL schema"))
        .get_matches();

    if let Some(ref matches) = matches.subcommand_matches("schema") {
        schema::execute(*matches).unwrap();
    }
}
