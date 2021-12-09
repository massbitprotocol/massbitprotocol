use structopt::StructOpt;
#[derive(Clone, Debug, StructOpt)]
#[structopt(
    name = "massbit-graphql",
    about = "Massbit graphql server for indexed data",
    author = "Massbit Team.",
    version = "0.1"
)]
pub struct Opt {
    #[structopt(
        long,
        default_value = "8000",
        value_name = "PORT",
        help = "Port for the GraphQL HTTP server"
    )]
    pub http_port: u16,
    #[structopt(long, help = "Enable debug logging")]
    pub debug: bool,
}
