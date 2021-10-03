use slog::*;

use crate::components::store::DeploymentLocator;

/// Factory for creating component and indexer loggers.
#[derive(Clone)]
pub struct LoggerFactory {
    parent: Logger,
}

impl LoggerFactory {
    /// Creates a new factory using a parent logger.
    pub fn new(logger: Logger) -> Self {
        Self { parent: logger }
    }

    /// Creates a new factory with a new parent logger.
    pub fn with_parent(&self, parent: Logger) -> Self {
        Self { parent }
    }

    /// Creates a component-specific logger.
    pub fn component_logger(&self, component: &str) -> Logger {
        self.parent.new(o!("component" => component.to_string()))
    }

    /// Creates a indexer logger.
    pub fn indexer_logger(&self, loc: &DeploymentLocator) -> Logger {
        self.parent
            .new(o!("indexer_id" => loc.hash.to_string(), "sgd" => loc.id.to_string()))
    }
}
