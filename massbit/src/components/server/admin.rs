use std::io;
use std::sync::Arc;

use crate::prelude::NodeId;

/// Common trait for JSON-RPC admin server implementations.
pub trait JsonRpcServer<R> {
    type Server;

    fn serve(port: u16, registrar: Arc<R>, node_id: NodeId) -> Result<Self::Server, io::Error>;
}
