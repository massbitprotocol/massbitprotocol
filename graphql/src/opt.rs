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
        env = "DATABASE_URL",
        conflicts_with = "config",
        required_unless = "config",
        help = "Location of the Postgres database used for storing entities"
    )]
    pub postgres_url: Option<String>,
    #[structopt(
        long,
        default_value = "8000",
        value_name = "HTTP_PORT",
        env = "HTTP_PORT",
        help = "Port for the GraphQL HTTP server"
    )]
    pub http_port: u16,
    #[structopt(
        long,
        default_value = "8001",
        value_name = "WS_PORT",
        env = "WS_PORT",
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
        value_name = "CONNECTION_POOL_SIZE",
        default_value = "10",
        env = "CONNECTION_POOL_SIZE",
        help = "Limits the number of connections in the store's connection pool"
    )]
    pub connection_pool_size: u32,
    #[structopt(
        long,
        value_name = "ACCESS_CONTROL_ALLOW_HEADERS",
        default_value = "Content-Type, User-Agent, Authorization, Access-Control-Allow-Origin",
        env = "ACCESS_CONTROL_ALLOW_HEADERS",
        help = "List of access control allow headers"
    )]
    pub access_control_allow_headers: String,
    #[structopt(
        long,
        value_name = "ACCESS_CONTROL_ALLOW_ORIGIN",
        default_value = "*",
        env = "ACCESS_CONTROL_ALLOW_ORIGIN",
        help = "List of access control allow origin"
    )]
    pub access_control_allow_origin: String,
    #[structopt(
        long,
        value_name = "ACCESS_CONTROL_ALLOW_METHODS",
        default_value = "CGET, OPTIONS, POST",
        env = "ACCESS_CONTROL_ALLOW_METHODS",
        help = "List of access control allow methods"
    )]
    pub access_control_allow_methods: String,
    #[structopt(
        long,
        value_name = "CONTENT_TYPE",
        default_value = "text/html",
        env = "CONTENT_TYPE",
        help = "Content type"
    )]
    pub content_type: String,
}

impl From<Opt> for config::Opt {
    fn from(opt: Opt) -> Self {
        let Opt {
            postgres_url,
            config,
            connection_pool_size,
            node_id,
            http_port,
            ws_port,
            debug,
            ..
        } = opt;
        config::Opt {
            postgres_url,
            config,
            connection_pool_size,
            postgres_secondary_hosts: vec![],
            postgres_host_weights: vec![],
            node_id,
            http_port: 0,
            ws_port: 0,
            debug: false,
        }
    }
}

impl From<&Opt> for config::AccessControl {
    fn from(opt: &Opt) -> Self {
        let Opt {
            access_control_allow_headers,
            access_control_allow_origin,
            access_control_allow_methods,
            content_type,
            ..
        } = opt;
        config::AccessControl {
            access_control_allow_headers: access_control_allow_headers.clone(),
            access_control_allow_origin: access_control_allow_origin.clone(),
            access_control_allow_methods: access_control_allow_methods.clone(),
            content_type: content_type.clone(),
        }
    }
}
