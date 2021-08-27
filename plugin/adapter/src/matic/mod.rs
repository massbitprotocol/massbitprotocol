use crate::core::MessageHandler;
pub use crate::stream_mod::{DataType, GenericDataProto};
use libloading::Library;
use std::{error::Error, sync::Arc};

crate::prepare_adapter!(Matic, {});

impl MessageHandler for MaticHandlerProxy {
    fn handle_rust_mapping(&self, _data: &mut GenericDataProto) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
