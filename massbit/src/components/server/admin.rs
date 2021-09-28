use std::io;
use std::sync::Arc;

use crate::prelude::NodeId;

/// Common trait for JSON-RPC admin server implementations.
pub trait JsonRpcServer<P> {
    type Server;

    fn serve(port: u16, provider: Arc<P>, node_id: NodeId) -> Result<Self::Server, io::Error>;
}
