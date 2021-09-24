use crate::core::MessageHandler;
pub use crate::stream_mod::{DataType, GenericDataProto};
use index_store::Store;
use libloading::Library;
use std::{error::Error, sync::Arc};
// crate::prepare_adapter!(Bsc, {});

lazy_static::lazy_static! {
    static ref COMPONENT_NAME: String = String::from(format!("[{}-Adapter]", quote::quote!(Bsc)));
}

pub trait BscHandler {}

pub struct BscHandlerProxy {
    pub handler: Box<dyn BscHandler + Send + Sync>,
    _lib: Arc<Library>,
}

impl BscHandlerProxy {
    pub fn new(handler: Box<dyn BscHandler + Send + Sync>, _lib: Arc<Library>) -> BscHandlerProxy {
        BscHandlerProxy { handler, _lib }
    }
}
impl BscHandler for BscHandlerProxy {}
impl MessageHandler for BscHandlerProxy {
    fn handle_rust_mapping(
        &self,
        _data: &mut GenericDataProto,
        store: &mut dyn Store,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
