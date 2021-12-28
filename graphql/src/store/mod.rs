use massbit_common::prelude::lazy_static::lazy_static;
use std::env;
use std::str::FromStr;
use std::time::Duration;

pub mod prefetch;
pub mod query;
pub mod resolver;

pub use query::build_query;
pub use resolver::StoreResolver;

lazy_static! {
    pub static ref SUBSCRIPTION_THROTTLE_INTERVAL: Duration =
        env::var("SUBSCRIPTION_THROTTLE_INTERVAL")
            .ok()
            .map(|s| u64::from_str(&s).unwrap_or_else(|_| panic!(
                "failed to parse env var SUBSCRIPTION_THROTTLE_INTERVAL"
            )))
            .map(Duration::from_millis)
            .unwrap_or_else(|| Duration::from_millis(1000));
}
