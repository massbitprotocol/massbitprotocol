use crate::core::MessageHandler;
use index_store::Store;
use libloading::Library;
pub use massbit::firehose::dstream::{DataType, GenericDataProto};
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
