pub mod prelude {
    pub use anyhow;
    //pub use anyhow::{anyhow, Context as _, Error};
    pub use async_trait;
    pub use diesel;
    pub use diesel_derives;
    pub use ethabi;
    pub use lazy_static;
    pub use log;
    pub use regex;
    pub use serde;
    pub use serde_derive;
    pub use serde_json;
    pub use serde_regex;
    pub use serde_yaml;
    pub use structmap;
    pub use tokio;
    pub use tokio_postgres;
}
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
