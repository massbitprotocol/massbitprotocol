use crate::config;
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
    env = "CONFIG",
    conflicts_with_all = &["postgres-url", "postgres-secondary-hosts", "postgres-host-weights"],
    required_unless = "postgres-url",
    help = "the name of the configuration file",
    )]
    pub config: Option<String>,
    #[structopt(
        long,
        value_name = "URL",
        env = "POSTGRES_URL",
        conflicts_with = "config",
        required_unless = "config",
        help = "Location of the Postgres database used for storing entities"
    )]
    pub postgres_url: Option<String>,
    #[structopt(
        long,
        default_value = "3031",
        value_name = "PORT",
        help = "Port for the GraphQL HTTP server"
    )]
    pub http_port: u16,
    #[structopt(
        long,
        default_value = "3032",
        value_name = "PORT",
        help = "Port for the GraphQL WebSocket server"
    )]
    pub ws_port: u16,
    #[structopt(long, help = "Enable debug logging")]
    pub debug: bool,
    #[structopt(
        long,
        default_value = "default",
        value_name = "NODE_ID",
        env = "INDEXER_NODE_ID",
        help = "a unique identifier for this node"
    )]
    pub node_id: String,
    #[structopt(
        long,
        value_name = "STORE_CONNECTION_POOL_SIZE",
        default_value = "10",
        env = "STORE_CONNECTION_POOL_SIZE",
        help = "Limits the number of connections in the store's connection pool"
    )]
    pub store_connection_pool_size: u32,
}

impl From<Opt> for config::Opt {
    fn from(opt: Opt) -> Self {
        let Opt {
            postgres_url,
            config,
            store_connection_pool_size,
            node_id,
            ..
        } = opt;
        config::Opt {
            postgres_url,
            config,
            store_connection_pool_size,
            postgres_secondary_hosts: vec![],
            postgres_host_weights: vec![],
            node_id,
        }
    }
}
