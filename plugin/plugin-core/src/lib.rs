use diesel::pg::PgConnection;
use diesel::prelude::*;
use types::{SubstrateBlock, SubstrateEvent, SubstrateExtrinsic};

pub trait BlockHandler {
    fn handle_block(&self, block: &SubstrateBlock) -> Result<(), InvocationError>;
}

pub trait ExtrinsicHandler {
    fn handle_extrinsic(&self, block: &SubstrateExtrinsic) -> Result<(), InvocationError>;
}

pub trait EventHandler {
    fn handle_event(&self, block: &SubstrateEvent) -> Result<(), InvocationError>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum InvocationError {
    InvalidArgumentCount { expected: usize, found: usize },
    Other { msg: String },
}

impl<S: ToString> From<S> for InvocationError {
    fn from(other: S) -> InvocationError {
        InvocationError::Other {
            msg: other.to_string(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct PluginDeclaration {
    pub register: unsafe extern "C" fn(&mut dyn PluginRegistrar),
}

pub trait PluginRegistrar {
    fn register_block_handler(&mut self, name: &str, function: Box<dyn BlockHandler>);
    fn register_extrinsic_handler(&mut self, name: &str, function: Box<dyn ExtrinsicHandler>);
    fn register_event_handler(&mut self, name: &str, function: Box<dyn EventHandler>);
}

#[macro_export]
macro_rules! export_plugin {
    ($register:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub static plugin_declaration: $crate::PluginDeclaration = $crate::PluginDeclaration {
            register: $register,
        };
    };
}
