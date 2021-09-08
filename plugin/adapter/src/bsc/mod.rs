use crate::core::MessageHandler;
pub use crate::stream_mod::{DataType, GenericDataProto};
use index_store::Store;
use libloading::Library;
use std::{error::Error, sync::Arc};
crate::prepare_adapter!(Bsc, {});

impl MessageHandler for BscHandlerProxy {
    fn handle_rust_mapping(
        &self,
        _data: &mut GenericDataProto,
        store: &mut dyn Store,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
