pub mod adapter;
pub mod data_source;
pub mod ethereum_adapter;
pub mod transport;
pub mod trigger;

pub mod chain;
#[path = "test/manifest.rs"]
pub mod manifest;
pub mod network;
pub mod runtime;

pub use self::ethereum_adapter::EthereumAdapter;
pub use self::runtime::RuntimeAdapter;
pub use self::transport::{EventLoopHandle, Transport};
pub use crate::adapter::{
    EthereumAdapter as EthereumAdapterTrait, EthereumContractCall, EthereumContractCallError,
    TriggerFilter,
};
pub use crate::chain::Chain;

// ETHDEP: These concrete types should probably not be exposed.
pub use data_source::{DataSource, DataSourceTemplate, Mapping, MappingABI, TemplateSource};
pub use trigger::MappingTrigger;
