use crate::core::MessageHandler;
pub use crate::stream_mod::{DataType, GenericDataProto};
use index_store::Store;
use libloading::Library;
use std::{error::Error, sync::Arc};

// crate::prepare_adapter!(Matic, {});

lazy_static::lazy_static! {
    static ref COMPONENT_NAME: String = String::from(format!("[{}-Adapter]", quote::quote!(Matic)));
}

pub trait MaticHandler {}

pub struct MaticHandlerProxy {
    pub handler: Box<dyn MaticHandler + Send + Sync>,
    _lib: Arc<Library>,
}

impl MaticHandlerProxy {
    pub fn new(
        handler: Box<dyn MaticHandler + Send + Sync>,
        _lib: Arc<Library>,
    ) -> MaticHandlerProxy {
        MaticHandlerProxy { handler, _lib }
    }
}
impl MaticHandler for MaticHandlerProxy {}

impl MessageHandler for MaticHandlerProxy {
    fn handle_rust_mapping(
        &self,
        _data: &mut GenericDataProto,
        store: &mut dyn Store,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
