pub mod plugin;
pub mod store;
pub mod types;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[Solana-SDK]");
}
pub mod entity {
    pub use massbit_data::store::{Attribute, Entity, Value};
}
