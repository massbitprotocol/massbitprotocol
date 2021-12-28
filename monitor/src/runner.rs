use massbit_common::prelude::async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait MonitorRunner: Send + Sync + 'static {
    async fn handle_request(self: Arc<Self>);
}
