use std::iter::FromIterator;
use std::{collections::HashMap, sync::Arc};

use crate::postgres::indexer_store::IndexerStore;
use massbit_common::cheap_clone::CheapClone;
use massbit_common::prelude::slog::{info, o, Logger};
use massbit_common::prelude::tokio_postgres::Config;
use massbit_data::metrics::registry::MetricsRegistry;
use massbit_data::metrics::MetricsRegistry as MetricsRegistryTrait;

pub struct StoreBuilder {
    logger: Logger,
}

impl StoreBuilder {
    /// Set up all stores, and run migrations. This does a complete store
    /// setup whereas other methods here only get connections for an already
    /// initialized store
    pub async fn new(logger: &Logger, registry: Arc<dyn MetricsRegistryTrait>) -> Self {
        Self {
            logger: logger.cheap_clone(),
        }
    }
    pub async fn indexer_store(&self) -> IndexerStore {
        IndexerStore {}
    }
}
