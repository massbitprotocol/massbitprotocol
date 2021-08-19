use massbit_runtime_wasm::chain::ethereum::data_source::{DataSource, DataSourceTemplate};
use massbit_runtime_wasm::chain::ethereum::Chain as Ethereum;
use massbit_runtime_wasm::graph::HostMetrics;
use massbit_runtime_wasm::indexer::DeploymentHash;
use massbit_runtime_wasm::mapping::{MappingContext, ValidModule};
use massbit_runtime_wasm::module::WasmInstance;
use massbit_runtime_wasm::prelude::{Arc, EntityType, Logger, MetricsRegistry};
use massbit_runtime_wasm::slog;
use test_plugin::common::{mock_context, mock_data_source};

use ethabi::ethereum_types::{H256, U64};
use massbit_runtime_wasm::asc_abi::class::{AscBigInt, AscEntity};
use massbit_runtime_wasm::chain::ethereum::trigger::MappingTrigger;
use massbit_runtime_wasm::chain::ethereum::trigger::MappingTrigger::Block;
use massbit_runtime_wasm::chain::ethereum::types::LightEthereumBlock;
use massbit_runtime_wasm::graph::components::store::{EntityCache, EntityModification};
use massbit_runtime_wasm::graph::data::store::{scalar, Entity};
use massbit_runtime_wasm::graph::prelude::StopwatchMetrics;
use massbit_runtime_wasm::graph::runtime::{asc_get, asc_new, try_asc_get, AscPtr};
use massbit_runtime_wasm::indexer::manifest::MappingBlockHandler;
use massbit_runtime_wasm::mock::MockMetricsRegistry;
use massbit_runtime_wasm::prelude::Value;
use semver::Version;
use slog::o;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::time::Duration;
use test_plugin::types::PairCreated;
use tokio;
use wasmtime;

const QUICKSWAP_PATH: &str = r#"/home/viettai/Massbit/QuickSwap-subgraph/build"#;
const WASM_FILE: &str = r#"Factory/Factory.wasm"#;

const API_VERSION_0_0_4: Version = Version::new(0, 0, 4);
const API_VERSION_0_0_5: Version = Version::new(0, 0, 5);

fn main() {
    println!("Test quickswap");
    let wasm_file_path = format!("{}/{}", QUICKSWAP_PATH, WASM_FILE);
    let data_source = mock_data_source(wasm_file_path.as_str(), API_VERSION_0_0_4.clone());
    let mut wasm_instance = create_wasm_instance(
        "quickswap",
        wasm_file_path,
        data_source,
        API_VERSION_0_0_4.clone(),
    );

    let block = LightEthereumBlock {
        hash: Some(H256::from_low_u64_be(123)),
        parent_hash: Default::default(),
        uncles_hash: Default::default(),
        author: Default::default(),
        state_root: Default::default(),
        transactions_root: Default::default(),
        receipts_root: Default::default(),
        number: Some(U64::from(834127)),
        gas_used: Default::default(),
        gas_limit: Default::default(),
        base_fee_per_gas: None,
        extra_data: Default::default(),
        logs_bloom: None,
        timestamp: Default::default(),
        difficulty: Default::default(),
        total_difficulty: None,
        seal_fields: vec![],
        uncles: vec![],
        transactions: vec![],
        size: None,
        mix_hash: None,
        nonce: None,
    };
    let trigger = MappingTrigger::Block {
        block: Arc::new(block),
        handler: MappingBlockHandler {
            handler: "handleNewPair".to_string(),
            filter: None,
        },
    };
    wasm_instance.handle_trigger(trigger);
    /*
    let handle_new_block =
        |module: &mut WasmInstance<Ethereum>, func_name: &str, block: &LightEthereumBlock| {
            let block_ptr = asc_new(module, block).unwrap();
            let func = module.get_func(func_name).typed().unwrap().clone();
            func.call((block_ptr.wasm_ptr()))
        };
    handle_new_block(&mut wasm_instance, "handleNewPair", &block);
     */
}

fn wasm_file_path(wasm_file: &str, api_version: Version) -> String {
    format!(
        "wasm_test/api_version_{}_{}_{}/{}",
        api_version.major, api_version.minor, api_version.patch, wasm_file
    )
}
fn indexer_id_with_api_version(indexer_id: &str, api_version: &Version) -> String {
    format!(
        "{}_{}_{}_{}",
        indexer_id, api_version.major, api_version.minor, api_version.patch
    )
}

fn create_wasm_instance(
    indexer_id: &str,
    wasm_file: String,
    data_source: DataSource,
    api_version: Version,
) -> WasmInstance<Ethereum> {
    let metrics_registry = Arc::new(MockMetricsRegistry::new());
    //let indexer_id_with_api_version = indexer_id_with_api_version(indexer_id, &api_version);
    //let deployment_id = DeploymentHash::new(&indexer_id_with_api_version).unwrap();
    let stopwatch_metrics = StopwatchMetrics::new(
        Logger::root(slog::Discard, o!()),
        indexer_id.to_string(),
        metrics_registry.clone(),
    );
    let host_metrics = Arc::new(HostMetrics::new(
        metrics_registry,
        indexer_id,
        stopwatch_metrics,
    ));
    let timeout = None;
    let valid_module = ValidModule::from_file(wasm_file.as_ref()).unwrap();
    println!(
        "import name to modules {:?}",
        &valid_module.import_name_to_modules
    );
    WasmInstance::from_valid_module_with_ctx(
        Arc::new(valid_module),
        mock_context(indexer_id, data_source, api_version),
        host_metrics,
        timeout,
        //experimental_features,
    )
    .unwrap()
}
trait WasmInstanceExt {
    fn invoke_export0(&self, f: &str);
    fn invoke_export<C, R>(&self, f: &str, arg: AscPtr<C>) -> AscPtr<R>;
    fn invoke_export2<C, D, R>(&self, f: &str, arg0: AscPtr<C>, arg1: AscPtr<D>) -> AscPtr<R>;
    fn invoke_export2_void<C, D>(
        &self,
        f: &str,
        arg0: AscPtr<C>,
        arg1: AscPtr<D>,
    ) -> Result<(), wasmtime::Trap>;
    fn takes_ptr_returns_val<P, V: wasmtime::WasmTy>(&mut self, fn_name: &str, v: AscPtr<P>) -> V;
    fn takes_val_returns_ptr<P>(&mut self, fn_name: &str, val: impl wasmtime::WasmTy) -> AscPtr<P>;
}

impl WasmInstanceExt for WasmInstance<Ethereum> {
    fn invoke_export0(&self, f: &str) {
        let func = self.get_func(f).typed().unwrap().clone();
        let _: () = func.call(()).unwrap();
    }

    fn invoke_export<C, R>(&self, f: &str, arg: AscPtr<C>) -> AscPtr<R> {
        let func = self.get_func(f).typed().unwrap().clone();
        let ptr: u32 = func.call(arg.wasm_ptr()).unwrap();
        ptr.into()
    }

    fn invoke_export2<C, D, R>(&self, f: &str, arg0: AscPtr<C>, arg1: AscPtr<D>) -> AscPtr<R> {
        let func = self.get_func(f).typed().unwrap().clone();
        let ptr: u32 = func.call((arg0.wasm_ptr(), arg1.wasm_ptr())).unwrap();
        ptr.into()
    }

    fn invoke_export2_void<C, D>(
        &self,
        f: &str,
        arg0: AscPtr<C>,
        arg1: AscPtr<D>,
    ) -> Result<(), wasmtime::Trap> {
        let func = self.get_func(f).typed().unwrap().clone();
        func.call((arg0.wasm_ptr(), arg1.wasm_ptr()))
    }

    fn takes_ptr_returns_val<P, V: wasmtime::WasmTy>(&mut self, fn_name: &str, v: AscPtr<P>) -> V {
        let func = self.get_func(fn_name).typed().unwrap().clone();
        func.call(v.wasm_ptr()).unwrap()
    }

    fn takes_val_returns_ptr<P>(&mut self, fn_name: &str, val: impl wasmtime::WasmTy) -> AscPtr<P> {
        let func = self.get_func(fn_name).typed().unwrap().clone();
        let ptr: u32 = func.call(val).unwrap();
        ptr.into()
    }
}
