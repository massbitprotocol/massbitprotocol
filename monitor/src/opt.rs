use structopt::StructOpt;

#[derive(Debug, Clone, Default)]
pub struct AccessControl {
    pub access_control_allow_headers: String,
    pub access_control_allow_origin: String,
    pub access_control_allow_methods: String,
    pub content_type: String,
}

#[derive(Clone, Debug, StructOpt)]
#[structopt(
    name = "indexer-monitor",
    about = "Massbit indexer monitor",
    author = "Massbit Team.",
    version = "0.1"
)]
pub struct Opt {
    #[structopt(
        long,
        default_value = "8000",
        value_name = "HTTP_PORT",
        env = "HTTP_PORT",
        help = "Port for the HTTP server"
    )]
    pub http_port: u16,
    #[structopt(
        long,
        default_value = "10000",
        value_name = "MONITOR_PERIOD",
        env = "MONITOR_PERIOD",
        help = "Period of monitoring indexers"
    )]
    pub monitor_period: u64,
    #[structopt(
        long,
        default_value = "10000",
        value_name = "DATABASE_URL",
        env = "DATABASE_URL",
        help = "Database url connection"
    )]
    pub database_url: String,
    #[structopt(
        long,
        default_value = "20",
        value_name = "CONNECTION_POOL_SIZE",
        env = "CONNECTION_POOL_SIZE",
        help = "Database connection pool size"
    )]
    pub pool_size: u32,
}
