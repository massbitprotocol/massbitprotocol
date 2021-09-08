use massbit_runtime_wasm::chain::ethereum::data_source::{DataSource, DataSourceTemplate};
use massbit_runtime_wasm::chain::ethereum::Chain as Ethereum;
use massbit_runtime_wasm::graph::components::metrics::{
    Collector, Counter, Gauge, MetricsRegistry, Opts,
};
use massbit_runtime_wasm::host_exports::HostExports;
use massbit_runtime_wasm::indexer::manifest::{Link, Mapping, MappingABI, Source, TemplateSource};
use massbit_runtime_wasm::indexer::types::BlockPtr;
use massbit_runtime_wasm::indexer::IndexerState;
use massbit_runtime_wasm::mapping::MappingContext;
use massbit_runtime_wasm::prelude::web3::ethabi::{Address, Contract};
use massbit_runtime_wasm::prelude::{Arc, Logger, PrometheusError};
use massbit_runtime_wasm::slog;
use massbit_runtime_wasm::store::IndexStore;
use semver::Version;
use slog::o;
use std::collections::HashMap;
use std::str::FromStr;

fn mock_host_exports(
    indexer_id: &str,
    data_source: DataSource,
    //store: Arc<impl SubgraphStore>,
    api_version: Version,
) -> HostExports<Ethereum> {
    let templates = vec![DataSourceTemplate {
        kind: String::from("ethereum/contract"),
        name: String::from("example template"),
        network: Some(String::from("mainnet")),
        source: TemplateSource {
            abi: String::from("foo"),
        },
        mapping: Mapping {
            kind: String::from("ethereum/events"),
            api_version: api_version.clone(),
            language: String::from("wasm/assemblyscript"),
            entities: vec![],
            abis: vec![],
            event_handlers: vec![],
            call_handlers: vec![],
            block_handlers: vec![],
            link: Link {
                link: "link".to_owned(),
            },
            runtime: Arc::new(vec![]),
        },
    }];

    let network = data_source.network.clone().unwrap();
    HostExports::new(
        indexer_id,
        &data_source,
        network,
        Arc::new(templates),
        api_version,
        //Arc::new(graph_core::LinkResolver::from(IpfsClient::localhost())),
        //store,
    )
}

fn mock_abi() -> MappingABI {
    MappingABI {
        name: "mock_abi".to_string(),
        contract: Contract::load(
            r#"[
            {
                "inputs": [
                    {
                        "name": "a",
                        "type": "address"
                    }
                ],
                "type": "constructor"
            }
        ]"#
            .as_bytes(),
        )
        .unwrap(),
    }
}

pub fn mock_context(
    indexer_id: &str,
    //deployment: DeploymentLocator,
    data_source: DataSource,
    //store: Arc<impl SubgraphStore>,
    api_version: Version,
) -> MappingContext<Ethereum> {
    let host_exports = mock_host_exports(indexer_id, data_source, api_version);
    MappingContext {
        logger: Logger::root(slog::Discard, o!()),
        block_ptr: BlockPtr {
            hash: Default::default(),
            number: 0,
        },
        host_exports: Arc::new(host_exports),
        state: IndexerState::new(Arc::new(IndexStore::new()), Default::default()),
        /*
        state: BlockState::new(store.writable(&deployment).unwrap(), Default::default()),
        proof_of_indexing: None,
        host_fns: Arc::new(Vec::new()),
         */
    }
}

pub fn mock_data_source(path: &str, api_version: Version) -> DataSource {
    let runtime = std::fs::read(path).unwrap();

    DataSource {
        kind: String::from("ethereum/contract"),
        name: String::from("example data source"),
        network: Some(String::from("mainnet")),
        source: Source {
            address: Some(Address::from_str("0123123123012312312301231231230123123123").unwrap()),
            abi: String::from("123123"),
            start_block: 0,
        },
        mapping: Mapping {
            kind: String::from("ethereum/events"),
            api_version,
            language: String::from("wasm/assemblyscript"),
            entities: vec![],
            abis: vec![],
            event_handlers: vec![],
            call_handlers: vec![],
            block_handlers: vec![],
            link: Link {
                link: "link".to_owned(),
            },
            runtime: Arc::new(runtime.clone()),
        },
        context: Default::default(),
        creation_block: None,
        contract_abi: Arc::new(mock_abi()),
    }
}
