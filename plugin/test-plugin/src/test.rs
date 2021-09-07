use crate::common::{mock_context, MockMetricsRegistry};
use massbit_runtime_wasm::chain::ethereum::data_source::{DataSource, DataSourceTemplate};
use massbit_runtime_wasm::chain::ethereum::Chain as Ethereum;
use massbit_runtime_wasm::graph::HostMetrics;
use massbit_runtime_wasm::indexer::DeploymentHash;
use massbit_runtime_wasm::mapping::{MappingContext, ValidModule};
use massbit_runtime_wasm::module::WasmInstance;
use massbit_runtime_wasm::prelude::{Arc, EntityType, Logger, MetricsRegistry};
use massbit_runtime_wasm::slog;

use massbit_runtime_wasm::asc_abi::class::{AscBigInt, AscEntity};
use massbit_runtime_wasm::graph::components::store::{EntityCache, EntityModification};
use massbit_runtime_wasm::graph::data::store::{scalar, Entity};
use massbit_runtime_wasm::graph::prelude::StopwatchMetrics;
use massbit_runtime_wasm::graph::runtime::{asc_get, asc_new, try_asc_get, AscPtr};
use massbit_runtime_wasm::mock::MockMetricsRegistry;
use massbit_runtime_wasm::prelude::Value;
use semver::Version;
use slog::o;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::time::Duration;
use tokio;
use wasmtime;

const API_VERSION_0_0_4: Version = Version::new(0, 0, 4);
const API_VERSION_0_0_5: Version = Version::new(0, 0, 5);

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
/*
fn test_valid_module_and_store(
    indexer_id: &str,
    data_source: DataSource,
    api_version: Version,
) -> (WasmInstance, Arc<impl SubgraphStore>, DeploymentLocator) {
    test_valid_module_and_store_with_timeout(indexer_id, data_source, api_version, None)
}

fn test_valid_module_and_store_with_timeout(
    indexer_id: &str,
    data_source: DataSource,
    api_version: Version,
    timeout: Option<Duration>,
) -> (WasmInstance, Arc<impl SubgraphStore>, DeploymentLocator) {
    let indexer_id_with_api_version = indexer_id_with_api_version(indexer_id, api_version.clone());

    let store = STORE.clone();
    let metrics_registry = Arc::new(MockMetricsRegistry::new());
    let deployment_id = DeploymentHash::new(&indexer_id_with_api_version).unwrap();
    let deployment = test_store::create_test_subgraph(
        &deployment_id,
        "type User @entity {
            id: ID!,
            name: String,
        }

        type Thing @entity {
            id: ID!,
            value: String,
            extra: String
        }",
    );
    let stopwatch_metrics = StopwatchMetrics::new(
        Logger::root(slog::Discard, o!()),
        deployment_id.clone(),
        metrics_registry.clone(),
    );
    let host_metrics = Arc::new(HostMetrics::new(
        metrics_registry,
        deployment_id.as_str(),
        stopwatch_metrics,
    ));
    /*
    let experimental_features = ExperimentalFeatures {
        allow_non_deterministic_ipfs: true,
    };
    */
    let module = WasmInstance::from_valid_module_with_ctx(
        Arc::new(ValidModule::new(data_source.mapping.runtime.as_ref()).unwrap()),
        mock_context(
            deployment.clone(),
            data_source,
            store.subgraph_store(),
            api_version,
        ),
        host_metrics,
        timeout,
        //experimental_features,
    )
    .unwrap();

    (module, store.subgraph_store(), deployment)
}
 */
fn test_module(
    indexer_id: &str,
    wasm_file_name: &str,
    data_source: DataSource,
    api_version: Version,
) -> WasmInstance<Ethereum> {
    let wasm_file = wasm_file_path(wasm_file_name, api_version.clone());
    let metrics_registry = Arc::new(MockMetricsRegistry::new());
    let indexer_id_with_api_version = indexer_id_with_api_version(indexer_id, &api_version);
    let deployment_id = DeploymentHash::new(&indexer_id_with_api_version).unwrap();
    let stopwatch_metrics = StopwatchMetrics::new(
        Logger::root(slog::Discard, o!()),
        deployment_id.clone(),
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
fn test_json_conversions(api_version: Version) {
    let mut module = test_module(
        "jsonConversions",
        "string_to_number.wasm",
        api_version.clone(),
    );

    // test u64 conversion
    let number = 9223372036850770800;
    let number_ptr = asc_new(&mut module, &number.to_string()).unwrap();
    let converted: i64 = module.takes_ptr_returns_val("testToU64", number_ptr);
    assert_eq!(number, u64::from_le_bytes(converted.to_le_bytes()));

    // test i64 conversion
    let number = -9223372036850770800;
    let number_ptr = asc_new(&mut module, &number.to_string()).unwrap();
    let converted: i64 = module.takes_ptr_returns_val("testToI64", number_ptr);
    assert_eq!(number, converted);

    // test f64 conversion
    let number = -9223372036850770.92345034;
    let number_ptr = asc_new(&mut module, &number.to_string()).unwrap();
    let converted: f64 = module.takes_ptr_returns_val("testToF64", number_ptr);
    assert_eq!(number, converted);

    // test BigInt conversion
    let number = "-922337203685077092345034";
    let number_ptr = asc_new(&mut module, number).unwrap();
    let big_int_obj: AscPtr<AscBigInt> = module.invoke_export("testToBigInt", number_ptr);
    let bytes: Vec<u8> = asc_get(&module, big_int_obj).unwrap();
    assert_eq!(
        scalar::BigInt::from_str(number).unwrap(),
        scalar::BigInt::from_signed_bytes_le(&bytes)
    );
}
#[tokio::test]
async fn json_conversions_v0_0_4() {
    test_json_conversions(API_VERSION_0_0_4);
}

#[tokio::test]
async fn json_conversions_v0_0_5() {
    test_json_conversions(API_VERSION_0_0_5);
}

fn test_entity_store(api_version: Version) {
    /*
    let (mut module, store, deployment) = test_valid_module_and_store(
        "entityStore",
        mock_data_source(
            &wasm_file_path("store.wasm", api_version.clone()),
            api_version.clone(),
        ),
        api_version,
    );
    */
    let mut module = test_module("entityStore", "store.wasm", api_version.clone());
    let mut alex = Entity::new();
    alex.set("id", "alex");
    alex.set("name", "Alex");
    let mut steve = Entity::new();
    steve.set("id", "steve");
    steve.set("name", "Steve");
    let user_type = EntityType::from("User");
    /*
    test_store::insert_entities(
        &deployment,
        vec![(user_type.clone(), alex), (user_type, steve)],
    )
    .unwrap();
    */
    let get_user = move |module: &mut WasmInstance, id: &str| -> Option<Entity> {
        let id = asc_new(module, id).unwrap();
        let entity_ptr: AscPtr<AscEntity> = module.invoke_export("getUser", id);
        if entity_ptr.is_null() {
            None
        } else {
            Some(Entity::from(
                try_asc_get::<HashMap<String, Value>, _, _>(module, entity_ptr).unwrap(),
            ))
        }
    };

    let load_and_set_user_name = |module: &mut WasmInstance, id: &str, name: &str| {
        let id_ptr = asc_new(module, id).unwrap();
        let name_ptr = asc_new(module, name).unwrap();
        module
            .invoke_export2_void("loadAndSetUserName", id_ptr, name_ptr)
            .unwrap();
    };

    // store.get of a nonexistent user
    assert_eq!(None, get_user(&mut module, "herobrine"));
    // store.get of an existing user
    let steve = get_user(&mut module, "steve").unwrap();
    assert_eq!(Some(&Value::from("Steve")), steve.get("name"));

    // Load, set, save cycle for an existing entity
    load_and_set_user_name(&mut module, "steve", "Steve-O");
    /*
    // We need to empty the cache for the next test
    let writable = store.writable(&deployment).unwrap();
    let cache = std::mem::replace(
        &mut module.instance_ctx_mut().ctx.state.entity_cache,
        EntityCache::new(writable.clone()),
    );
    let mut mods = cache.as_modifications().unwrap().modifications;
    assert_eq!(1, mods.len());
    match mods.pop().unwrap() {
        EntityModification::Overwrite { data, .. } => {
            assert_eq!(Some(&Value::from("steve")), data.get("id"));
            assert_eq!(Some(&Value::from("Steve-O")), data.get("name"));
        }
        _ => assert!(false, "expected Overwrite modification"),
    }
    */
    // Load, set, save cycle for a new entity with fulltext API
    load_and_set_user_name(&mut module, "herobrine", "Brine-O");
    let mut fulltext_entities = BTreeMap::new();
    let mut fulltext_fields = BTreeMap::new();
    fulltext_fields.insert("name".to_string(), vec!["search".to_string()]);
    fulltext_entities.insert("User".to_string(), fulltext_fields);
    let mut mods = module
        .take_ctx()
        .ctx
        .state
        .entity_cache
        .as_modifications()
        .unwrap()
        .modifications;
    assert_eq!(1, mods.len());
    match mods.pop().unwrap() {
        EntityModification::Insert { data, .. } => {
            assert_eq!(Some(&Value::from("herobrine")), data.get("id"));
            assert_eq!(Some(&Value::from("Brine-O")), data.get("name"));
        }
        _ => assert!(false, "expected Insert modification"),
    };
}

#[tokio::test]
async fn entity_store_v0_0_4() {
    test_entity_store(API_VERSION_0_0_4);
}

#[tokio::test]
async fn entity_store_v0_0_5() {
    test_entity_store(API_VERSION_0_0_5);
}
