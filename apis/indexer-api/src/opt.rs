use crate::config;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
#[structopt(
    name = "indexer-api",
    about = "API for calling from font-end",
    author = "Massbit Team.",
    version = "0.1"
)]
pub struct Opt {
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
        default_value = "GET, OPTIONS, POST",
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
