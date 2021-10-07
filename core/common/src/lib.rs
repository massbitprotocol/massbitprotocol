pub mod consts;
pub mod prelude {
    pub use anyhow;
    //pub use anyhow::{anyhow, Context as _, Error};
    pub use async_trait;
    pub use bigdecimal;
    pub use bs58;
    pub use diesel;
    pub use diesel_derives;
    pub use env_logger;
    pub use ethabi;
    pub use lazy_static;
    pub use log;
    pub use r2d2;
    pub use r2d2_diesel;
    pub use regex;
    pub use reqwest;
    pub use serde;
    pub use serde_derive;
    pub use serde_json;
    pub use serde_regex;
    pub use serde_yaml;
    pub use slog;
    pub use structmap;
    pub use tokio;
    pub use tokio_compat_02;
    pub use tokio_postgres;
}

pub type NetworkType = String;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
