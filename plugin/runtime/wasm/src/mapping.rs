use crate::graph::prelude::CheapClone;
use crate::graph::runtime::AscHeap;
use crate::host_exports::HostExports;
use crate::indexer::blockchain::Blockchain;
use crate::indexer::types::BlockPtr;
use crate::indexer::IndexerState;
use crate::prelude::Logger;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

pub struct HostFnCtx<'a> {
    pub logger: Logger,
    //pub block_ptr: BlockPtr,
    pub heap: &'a mut dyn AscHeap,
}
/*
/// Host fn that receives one u32 argument and returns an u32.
/// The name for an AS fuction is in the format `<namespace>.<function>`.
#[derive(Clone)]
pub struct HostFn {
    pub name: &'static str,
    pub func: Arc<dyn Send + Sync + Fn(HostFnCtx, u32) -> Result<u32, HostExportError>>,
}

impl CheapClone for HostFn {
    fn cheap_clone(&self) -> Self {
        HostFn {
            name: self.name,
            func: self.func.cheap_clone(),
        }
    }
}
 */
pub struct MappingRequest<C: Blockchain> {
    pub(crate) ctx: MappingContext<C>,
    pub(crate) trigger: C::MappingTrigger,
    //pub(crate) result_sender: Sender<Result<BlockState<C>, MappingError>>,
}

pub struct MappingContext<C: Blockchain> {
    pub logger: Logger,
    pub host_exports: Arc<HostExports<C>>,
    pub block_ptr: BlockPtr,
    pub state: IndexerState<C>,
    //pub proof_of_indexing: SharedProofOfIndexing,
    //pub host_fns: Arc<Vec<HostFn>>,
}

impl<C: Blockchain> MappingContext<C> {
    pub fn derive_with_empty_block_state(&self) -> Self {
        MappingContext {
            logger: self.logger.cheap_clone(),
            host_exports: self.host_exports.cheap_clone(),
            state: IndexerState::new(self.state.entity_cache.store.clone(), Default::default()),
            block_ptr: self.block_ptr.cheap_clone(),
            //state: BlockState::new(self.state.entity_cache.store.clone(), Default::default()),
            //proof_of_indexing: self.proof_of_indexing.cheap_clone(),
            //host_fns: self.host_fns.cheap_clone(),
        }
    }
}
/// A pre-processed and valid WASM module, ready to be started as a WasmModule.
pub struct ValidModule {
    pub module: wasmtime::Module,

    // A wasm import consists of a `module` and a `name`. AS will generate imports such that they
    // have `module` set to the name of the file it is imported from and `name` set to the imported
    // function name or `namespace.function` if inside a namespace. We'd rather not specify names of
    // source files, so we consider that the import `name` uniquely identifies an import. Still we
    // need to know the `module` to properly link it, so here we map import names to modules.
    //
    // AS now has an `@external("module", "name")` decorator which would make things cleaner, but
    // the ship has sailed.
    pub import_name_to_modules: BTreeMap<String, Vec<String>>,
}

impl ValidModule {
    /// Pre-process and validate the module from binary.
    pub fn from_binary(raw_module: &[u8]) -> Result<Self, anyhow::Error> {
        let engine = create_engine()?;
        let module = wasmtime::Module::from_binary(&engine, raw_module)?;
        let mut import_name_to_modules: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // Unwrap: Module linking is disabled.
        for (name, module) in module
            .imports()
            .map(|import| (import.name().unwrap(), import.module()))
        {
            import_name_to_modules
                .entry(name.to_string())
                .or_default()
                .push(module.to_string());
        }

        Ok(ValidModule {
            module,
            import_name_to_modules,
        })
    }
    /// Pre-process and validate the module from binary.
    pub fn from_file(file_path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let engine = create_engine()?;
        let module = wasmtime::Module::from_file(&engine, file_path)?;

        let mut import_name_to_modules: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // Unwrap: Module linking is disabled.
        for (name, module) in module
            .imports()
            .map(|import| (import.name().unwrap(), import.module()))
        {
            import_name_to_modules
                .entry(name.to_string())
                .or_default()
                .push(module.to_string());
        }

        Ok(ValidModule {
            module,
            import_name_to_modules,
        })
    }
}
fn create_engine() -> Result<wasmtime::Engine, anyhow::Error> {
    // We currently use Cranelift as a compilation engine. Cranelift is an optimizing compiler,
    // but that should not cause determinism issues since it adheres to the Wasm spec. Still we
    // turn off optional optimizations to be conservative.
    let mut config = wasmtime::Config::new();
    config.strategy(wasmtime::Strategy::Cranelift).unwrap();
    config.interruptable(true); // For timeouts.
    config.cranelift_nan_canonicalization(true); // For NaN determinism.
    config.cranelift_opt_level(wasmtime::OptLevel::None);
    wasmtime::Engine::new(&config)
}
