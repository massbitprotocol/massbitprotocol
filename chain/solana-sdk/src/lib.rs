pub mod entity;
pub mod model;
pub mod plugin;
pub mod scalar;
pub mod store;
pub mod types;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[Solana-SDK]");
}
