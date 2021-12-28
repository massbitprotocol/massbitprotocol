pub mod effort;
pub mod ext;
pub mod object_macro;
pub mod object_or_interface;
pub mod serialization;
pub mod shape_hash;
pub mod values;

use crate::prelude::CacheStatus;
pub use effort::*;
pub use ext::*;
use massbit_common::prelude::async_trait::async_trait;
pub use object_macro::{object_value, IntoValue};
pub use object_or_interface::ObjectOrInterface;
pub use serialization::SerializableValue;
use std::time::Duration;
pub use values::TryFromValue;

#[async_trait]
pub trait QueryLoadManager: Send + Sync {
    fn record_work(&self, shape_hash: u64, duration: Duration, cache_status: CacheStatus);
}
