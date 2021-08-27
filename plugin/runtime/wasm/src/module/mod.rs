use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
    sync::Arc,
    time::Duration,
};
use wasmtime::{Memory, Trap};
pub mod context;
pub mod into_wasm_ret;
pub mod stopwatch;
use crate::prelude::{error, Version};
use crate::store;
pub use context::WasmInstanceContext;
use graph::{
    blockchain::{Blockchain, HostFnCtx, MappingTrigger},
    prelude::{CheapClone, HostMetrics, Value},
    runtime::{
        asc_new, AscHeap, AscPtr, DeterministicHostError, HostExportError, IndexForAscTypeId,
    },
};
use graph_runtime_wasm::error::DeterminismLevel;
pub use into_wasm_ret::IntoWasmRet;
use massbit_common::prelude::{
    anyhow::{anyhow, Context, Error},
    log::info,
    serde_json,
};

use crate::mapping::MappingContext;
use graph::components::subgraph::MappingError;
use graph::data::subgraph::schema::SubgraphError;
use graph::prelude::DeploymentHash;
use graph_runtime_wasm::module::TimeoutStopwatch;
use graph_runtime_wasm::ValidModule;
use std::sync::Mutex;
use std::time::Instant;

pub const TRAP_TIMEOUT: &str = "trap: interrupt";

pub trait IntoTrap {
    fn determinism_level(&self) -> DeterminismLevel;
    fn into_trap(self) -> Trap;
}
impl IntoTrap for DeterministicHostError {
    fn determinism_level(&self) -> DeterminismLevel {
        DeterminismLevel::Deterministic
    }
    fn into_trap(self) -> Trap {
        Trap::from(self.0)
    }
}
/// Handle to a WASM instance, which is terminated if and only if this is dropped.
pub struct WasmInstance<C: Blockchain> {
    pub instance: wasmtime::Instance,

    // This is the only reference to `WasmInstanceContext` that's not within the instance itself, so
    // we can always borrow the `RefCell` with no concern for race conditions.
    //
    // Also this is the only strong reference, so the instance will be dropped once this is dropped.
    // The weak references are circulary held by instance itself through host exports.
    pub instance_ctx: Rc<RefCell<Option<WasmInstanceContext<C>>>>,
}

impl<C: Blockchain> Drop for WasmInstance<C> {
    fn drop(&mut self) {
        //Assert that the instance will be dropped.
        assert_eq!(Rc::strong_count(&self.instance_ctx), 1);
    }
}
/// Proxies to the WasmInstanceContext.
impl<C: Blockchain> AscHeap for WasmInstance<C> {
    fn raw_new(&mut self, bytes: &[u8]) -> Result<u32, DeterministicHostError> {
        let mut ctx = RefMut::map(self.instance_ctx.borrow_mut(), |i| i.as_mut().unwrap());
        ctx.raw_new(bytes)
    }

    fn get(&self, offset: u32, size: u32) -> Result<Vec<u8>, DeterministicHostError> {
        self.instance_ctx().get(offset, size)
    }

    fn api_version(&self) -> Version {
        self.instance_ctx().api_version()
    }

    fn asc_type_id(
        &mut self,
        type_id_index: IndexForAscTypeId,
    ) -> Result<u32, DeterministicHostError> {
        self.instance_ctx_mut().asc_type_id(type_id_index)
    }
}
impl<C: Blockchain> WasmInstance<C> {
    pub(crate) fn handle_json_callback(
        mut self,
        handler_name: &str,
        value: &serde_json::Value,
        user_data: &graph::prelude::Value,
    ) -> Result<(), anyhow::Error> {
        let value = asc_new(&mut self, value)?;
        let user_data = asc_new(&mut self, user_data)?;
        //let mut state = Arc::clone(&self.instance_ctx_mut().ctx.state);
        //state.lock().unwrap().enter_handler();
        self.instance_ctx_mut().ctx.state.enter_handler();
        // Invoke the callback
        self.instance
            .get_func(handler_name)
            .with_context(|| format!("function {} not found", handler_name))?
            .typed()?
            .call((value.wasm_ptr(), user_data.wasm_ptr()))
            .with_context(|| format!("Failed to handle callback '{}'", handler_name))?;

        self.instance_ctx_mut().ctx.state.exit_handler();
        //state.lock().unwrap().exit_handler();
        //Ok(self.take_ctx().ctx.state)
        Ok(())
    }

    pub fn handle_trigger(&mut self, trigger: C::MappingTrigger) -> Result<(), MappingError> {
        let handler_name = trigger.handler_name().to_owned();
        let asc_trigger = trigger.to_asc_ptr(self)?;
        self.invoke_handler(&handler_name, asc_trigger)
    }

    pub fn take_ctx(&mut self) -> WasmInstanceContext<C> {
        self.instance_ctx.borrow_mut().take().unwrap()
    }

    pub(crate) fn instance_ctx(&self) -> std::cell::Ref<'_, WasmInstanceContext<C>> {
        match self.instance_ctx.borrow().as_ref() {
            Some(_) => {}
            _ => println!("Wasm instance don't has context"),
        };
        std::cell::Ref::map(self.instance_ctx.borrow(), |i| i.as_ref().unwrap())
    }

    pub fn instance_ctx_mut(&self) -> std::cell::RefMut<'_, WasmInstanceContext<C>> {
        std::cell::RefMut::map(self.instance_ctx.borrow_mut(), |i| i.as_mut().unwrap())
    }

    #[cfg(debug_assertions)]
    pub fn get_func(&self, func_name: &str) -> wasmtime::Func {
        self.instance.get_func(func_name).unwrap()
    }

    fn invoke_handler<T>(&mut self, handler: &str, arg: AscPtr<T>) -> Result<(), MappingError> {
        let func = self
            .instance
            .get_func(handler)
            .with_context(|| format!("function {} not found", handler))?;

        // Caution: Make sure all exit paths from this function call `exit_handler`.
        //let state = Arc::clone(&self.instance_ctx_mut().ctx.state);
        //state.lock().unwrap().enter_handler();
        self.instance_ctx_mut().ctx.state.enter_handler();

        // This `match` will return early if there was a non-deterministic trap.
        let deterministic_error: Option<Error> = match func.typed()?.call(arg.wasm_ptr()) {
            Ok(()) => None,
            Err(trap) if self.instance_ctx().possible_reorg => {
                self.instance_ctx_mut().ctx.state.exit_handler();
                //state.lock().unwrap().exit_handler();
                return Err(MappingError::PossibleReorg(trap.into()));
            }
            Err(trap) if trap.to_string().contains(TRAP_TIMEOUT) => {
                self.instance_ctx_mut().ctx.state.exit_handler();
                //state.lock().unwrap().exit_handler();
                return Err(MappingError::Unknown(Error::from(trap).context(format!(
                    "Handler '{}' hit the timeout",
                    //"Handler '{}' hit the timeout of '{}' seconds",
                    handler,
                    //self.instance_ctx().timeout.unwrap().as_secs()
                ))));
            }
            Err(trap) => {
                use wasmtime::TrapCode::*;
                let trap_code = trap.trap_code();
                let e = Error::from(trap);
                match trap_code {
                    Some(MemoryOutOfBounds)
                    | Some(HeapMisaligned)
                    | Some(TableOutOfBounds)
                    | Some(IndirectCallToNull)
                    | Some(BadSignature)
                    | Some(IntegerOverflow)
                    | Some(IntegerDivisionByZero)
                    | Some(BadConversionToInteger)
                    | Some(UnreachableCodeReached) => Some(e),
                    _ if self.instance_ctx().deterministic_host_trap => Some(e),
                    _ => {
                        self.instance_ctx_mut().ctx.state.exit_handler();
                        //state.lock().unwrap().exit_handler();
                        return Err(MappingError::Unknown(e));
                    }
                }
            }
        };

        if let Some(deterministic_error) = deterministic_error {
            let message = format!("{:#}", deterministic_error).replace("\n", "\t");

            // Log the error and restore the updates snapshot, effectively reverting the handler.
            error!(&self.instance_ctx().ctx.logger,
                "Handler skipped due to execution failure";
                "handler" => handler,
                "error" => &message,
            );

            let subgraph_error = SubgraphError {
                subgraph_id: DeploymentHash::new(
                    self.instance_ctx().ctx.host_exports.indexer_hash.clone(),
                )
                .unwrap(),
                message,
                block_ptr: Some(self.instance_ctx().ctx.block_ptr.cheap_clone()),
                handler: Some(handler.to_string()),
                deterministic: true,
            };
            println!("Error {:?}", deterministic_error);
            self.instance_ctx_mut()
                .ctx
                .state
                .exit_handler_and_discard_changes_due_to_error(subgraph_error);
        } else {
            self.instance_ctx_mut().ctx.state.exit_handler();
            //state.lock().unwrap().exit_handler();
        }
        //Ok(self.instance_ctx().ctx.state);
        Ok(())
    }
}

impl<C: Blockchain> WasmInstance<C> {
    /// Instantiates the module and sets it to be interrupted after `timeout`.
    pub fn from_valid_module_with_ctx(
        valid_module: Arc<ValidModule>,
        ctx: MappingContext<C>,
        host_metrics: Arc<HostMetrics>,
        timeout: Option<Duration>,
        //experimental_features: ExperimentalFeatures,
    ) -> Result<WasmInstance<C>, anyhow::Error> {
        let mut linker = wasmtime::Linker::new(&wasmtime::Store::new(valid_module.module.engine()));

        let host_fns = ctx.host_fns.cheap_clone();
        let api_version = ctx.host_exports.api_version.clone();

        // Used by exports to access the instance context. There are two ways this can be set:
        // - After instantiation, if no host export is called in the start function.
        // - During the start function, if it calls a host export.
        // Either way, after instantiation this will have been set.
        let shared_ctx: Rc<RefCell<Option<WasmInstanceContext<C>>>> = Rc::new(RefCell::new(None));
        // We will move the ctx only once, to init `shared_ctx`. But we don't statically know where
        // it will be moved so we need this ugly thing.
        let ctx: Rc<RefCell<Option<MappingContext<C>>>> = Rc::new(RefCell::new(Some(ctx)));
        // Start the timeout watchdog task.
        let timeout_stopwatch = Arc::new(std::sync::Mutex::new(TimeoutStopwatch::start_new()));

        macro_rules! link {
            ($wasm_name:expr, $rust_name:ident, $($param:ident),*) => {
                link!($wasm_name, $rust_name, "host_export_other", $($param),*)
            };

            ($wasm_name:expr, $rust_name:ident, $section:expr, $($param:ident),*) => {
                let modules = valid_module
                    .import_name_to_modules
                    .get($wasm_name)
                    .into_iter()
                    .flatten();

                // link an import with all the modules that require it.
                for module in modules {
                    let func_shared_ctx = Rc::downgrade(&shared_ctx);
                    let valid_module = valid_module.cheap_clone();
                    let host_metrics = host_metrics.cheap_clone();
                    let timeout_stopwatch = timeout_stopwatch.cheap_clone();
                    let ctx = ctx.cheap_clone();
                    linker.func(
                        module,
                        $wasm_name,
                        move |caller: wasmtime::Caller, $($param: u32),*| {
                            let instance = func_shared_ctx.upgrade().unwrap();
                            let mut instance = instance.borrow_mut();

                            // Happens when calling a host fn in Wasm start.
                            if instance.is_none() {
                                *instance = Some(WasmInstanceContext::from_caller(
                                    caller,
                                    ctx.borrow_mut().take().unwrap(),
                                    valid_module.cheap_clone(),
                                    host_metrics.cheap_clone(),
                                    timeout,
                                    timeout_stopwatch.cheap_clone(),
                                    //experimental_features.clone()
                                ).unwrap())
                            }
                            let instance = instance.as_mut().unwrap();
                            let _section = instance.host_metrics.stopwatch.start_section($section);

                            let result = instance.$rust_name(
                                $($param.into()),*
                            );
                            match result {
                                Ok(result) => Ok(result.into_wasm_ret()),
                                Err(e) => {
                                    match IntoTrap::determinism_level(&e) {
                                        DeterminismLevel::Deterministic => {
                                            instance.deterministic_host_trap = true;
                                        },
                                        DeterminismLevel::PossibleReorg => {
                                            instance.possible_reorg = true;
                                        },
                                        DeterminismLevel::Unimplemented | DeterminismLevel::NonDeterministic => {},
                                    }

                                    Err(IntoTrap::into_trap(e))
                                }
                            }
                        }
                    )?;
                }
            };
        }
        // Link chain-specifc host fns.
        for host_fn in host_fns.iter() {
            let modules = valid_module
                .import_name_to_modules
                .get(host_fn.name)
                .into_iter()
                .flatten();

            for module in modules {
                let func_shared_ctx = Rc::downgrade(&shared_ctx);
                let host_fn = host_fn.cheap_clone();
                linker.func(module, host_fn.name, move |call_ptr: u32| {
                    let start = Instant::now();
                    let instance = func_shared_ctx.upgrade().unwrap();
                    let mut instance = instance.borrow_mut();

                    let instance = match &mut *instance {
                        Some(instance) => instance,

                        // Happens when calling a host fn in Wasm start.
                        None => {
                            return Err(anyhow!(
                                "{} is not allowed in global variables",
                                host_fn.name
                            )
                            .into())
                        }
                    };

                    let name_for_metrics = host_fn.name.replace('.', "_");
                    let stopwatch = &instance.host_metrics.stopwatch;
                    let _section =
                        stopwatch.start_section(&format!("host_export_{}", name_for_metrics));

                    let ctx = HostFnCtx {
                        logger: instance.ctx.logger.cheap_clone(),
                        block_ptr: instance.ctx.block_ptr.cheap_clone(),
                        heap: instance,
                    };
                    let ret = (host_fn.func)(ctx, call_ptr).map_err(|e| match e {
                        HostExportError::Deterministic(e) => {
                            instance.deterministic_host_trap = true;
                            e
                        }
                        HostExportError::PossibleReorg(e) => {
                            instance.possible_reorg = true;
                            e
                        }
                        HostExportError::Unknown(e) => e,
                    })?;
                    instance.host_metrics.observe_host_fn_execution_time(
                        start.elapsed().as_secs_f64(),
                        &name_for_metrics,
                    );
                    Ok(ret)
                })?;
            }
        }
        link!("ethereum.encode", ethereum_encode, params_ptr);
        link!("ethereum.decode", ethereum_decode, params_ptr, data_ptr);
        link!("abort", abort, message_ptr, file_name_ptr, line, column);
        link!("store.get", store_get, "host_export_store_get", entity, id);
        link!(
            "store.set",
            store_set,
            "host_export_store_set",
            entity,
            id,
            data
        );

        link!("store.remove", store_remove, entity_ptr, id_ptr);

        link!("typeConversion.bytesToString", bytes_to_string, ptr);
        link!("typeConversion.bytesToHex", bytes_to_hex, ptr);
        link!("typeConversion.bigIntToString", big_int_to_string, ptr);
        link!("typeConversion.bigIntToHex", big_int_to_hex, ptr);
        link!("typeConversion.stringToH160", string_to_h160, ptr);
        link!("typeConversion.bytesToBase58", bytes_to_base58, ptr);

        link!("json.fromBytes", json_from_bytes, ptr);
        link!("json.try_fromBytes", json_try_from_bytes, ptr);
        link!("json.toI64", json_to_i64, ptr);
        link!("json.toU64", json_to_u64, ptr);
        link!("json.toF64", json_to_f64, ptr);
        link!("json.toBigInt", json_to_big_int, ptr);

        link!("crypto.keccak256", crypto_keccak_256, ptr);

        link!("bigInt.plus", big_int_plus, x_ptr, y_ptr);
        link!("bigInt.minus", big_int_minus, x_ptr, y_ptr);
        link!("bigInt.times", big_int_times, x_ptr, y_ptr);
        link!("bigInt.dividedBy", big_int_divided_by, x_ptr, y_ptr);
        link!("bigInt.dividedByDecimal", big_int_divided_by_decimal, x, y);
        link!("bigInt.mod", big_int_mod, x_ptr, y_ptr);
        link!("bigInt.pow", big_int_pow, x_ptr, exp);
        link!("bigInt.fromString", big_int_from_string, ptr);
        link!("bigInt.bitOr", big_int_bit_or, x_ptr, y_ptr);
        link!("bigInt.bitAnd", big_int_bit_and, x_ptr, y_ptr);
        link!("bigInt.leftShift", big_int_left_shift, x_ptr, bits);
        link!("bigInt.rightShift", big_int_right_shift, x_ptr, bits);

        link!("bigDecimal.toString", big_decimal_to_string, ptr);
        link!("bigDecimal.fromString", big_decimal_from_string, ptr);
        link!("bigDecimal.plus", big_decimal_plus, x_ptr, y_ptr);
        link!("bigDecimal.minus", big_decimal_minus, x_ptr, y_ptr);
        link!("bigDecimal.times", big_decimal_times, x_ptr, y_ptr);
        link!("bigDecimal.dividedBy", big_decimal_divided_by, x, y);
        link!("bigDecimal.equals", big_decimal_equals, x_ptr, y_ptr);

        link!("dataSource.create", data_source_create, name, params);
        link!(
            "dataSource.createWithContext",
            data_source_create_with_context,
            name,
            params,
            context
        );
        link!("dataSource.address", data_source_address,);
        link!("dataSource.network", data_source_network,);
        link!("dataSource.context", data_source_context,);

        link!("log.log", log_log, level, msg_ptr);
        let instance = linker.instantiate(&valid_module.module)?;
        // Usually `shared_ctx` is still `None` because no host fns were called during start.
        if shared_ctx.borrow().is_none() {
            info!("Create WasmInstanceContext from instance");
            *shared_ctx.borrow_mut() = Some(WasmInstanceContext::from_instance(
                &instance,
                ctx.borrow_mut().take().unwrap(),
                valid_module,
                host_metrics,
                timeout,
                timeout_stopwatch,
                //experimental_features,
            )?);
        }

        match api_version {
            version if version <= Version::new(0, 0, 4) => {}
            _ => {
                instance
                    .get_func("_start")
                    .context("`_start` function not found")?
                    .typed::<(), ()>()?
                    .call(())
                    .unwrap();
            }
        }
        Ok(WasmInstance {
            instance,
            instance_ctx: shared_ctx,
        })
    }
}
/*
impl<C: Blockchain> WasmInstance<C> {
    /// Instantiates the module and sets it to be interrupted after `timeout`.
    pub fn shared_from_valid_module_with_ctx(
        valid_module: Arc<ValidModule>,
        ctx: MappingContext<C>,
        host_metrics: Arc<HostMetrics>,
        timeout: Option<Duration>,
        //experimental_features: ExperimentalFeatures,
    ) -> Result<WasmInstance<C>, anyhow::Error> {
        let mut linker = wasmtime::Linker::new(&wasmtime::Store::new(valid_module.module.engine()));

        //let host_fns = ctx.host_fns.cheap_clone();
        let api_version = ctx.host_exports.api_version.clone();

        // Used by exports to access the instance context. There are two ways this can be set:
        // - After instantiation, if no host export is called in the start function.
        // - During the start function, if it calls a host export.
        // Either way, after instantiation this will have been set.
        let shared_ctx: Arc<Mutex<Option<WasmInstanceContext<C>>>> = Arc::new(Mutex::new(None));
        // We will move the ctx only once, to init `shared_ctx`. But we don't statically know where
        // it will be moved so we need this ugly thing.
        let ctx: Arc<Mutex<Option<MappingContext<C>>>> = Arc::new(Mutex::new(Some(ctx)));
        // Start the timeout watchdog task.
        let timeout_stopwatch = Arc::new(std::sync::Mutex::new(TimeoutStopwatch::start_new()));

        macro_rules! link {
            ($wasm_name:expr, $rust_name:ident, $($param:ident),*) => {
                link!($wasm_name, $rust_name, "host_export_other", $($param),*)
            };

            ($wasm_name:expr, $rust_name:ident, $section:expr, $($param:ident),*) => {
                let modules = valid_module
                    .import_name_to_modules
                    .get($wasm_name)
                    .into_iter()
                    .flatten();

                // link an import with all the modules that require it.
                for module in modules {
                    let func_shared_ctx = Arc::clone(&shared_ctx);
                    let valid_module = valid_module.cheap_clone();
                    let host_metrics = host_metrics.cheap_clone();
                    let timeout_stopwatch = timeout_stopwatch.cheap_clone();
                    let ctx = ctx.cheap_clone();
                    linker.func(
                        module,
                        $wasm_name,
                        move |caller: wasmtime::Caller, $($param: u32),*| {
                            let mut instance = func_shared_ctx.lock().unwrap();

                            // Happens when calling a host fn in Wasm start.
                            if instance.is_none() {
                                println!("Create WasmInstanceContext from caller");
                                let ctx_val = ctx.lock().unwrap();
                                *instance = Some(WasmInstanceContext::from_caller(
                                    caller,
                                    *ctx_val.unwrap(),
                                    valid_module.cheap_clone(),
                                    host_metrics.cheap_clone(),
                                    timeout,
                                    timeout_stopwatch.cheap_clone(),
                                    //experimental_features.clone()
                                ).unwrap())
                            }

                            let instance = instance.as_mut().unwrap();
                            let _section = instance.host_metrics.stopwatch.start_section($section);

                            let result = instance.$rust_name(
                                $($param.into()),*
                            );
                            match result {
                                Ok(result) => Ok(result.into_wasm_ret()),
                                Err(e) => {
                                    match IntoTrap::determinism_level(&e) {
                                        DeterminismLevel::Deterministic => {
                                            instance.deterministic_host_trap = true;
                                        },
                                        DeterminismLevel::PossibleReorg => {
                                            instance.possible_reorg = true;
                                        },
                                        DeterminismLevel::Unimplemented | DeterminismLevel::NonDeterministic => {},
                                    }

                                    Err(IntoTrap::into_trap(e))
                                }
                            }
                        }
                    )?;
                }
            };
        }
        link!("ethereum.call", ethereum_encode, params_ptr);
        link!("ethereum.encode", ethereum_encode, params_ptr);
        link!("ethereum.decode", ethereum_decode, params_ptr, data_ptr);
        link!("abort", abort, message_ptr, file_name_ptr, line, column);
        link!("store.get", store_get, "host_export_store_get", entity, id);
        link!(
            "store.set",
            store_set,
            "host_export_store_set",
            entity,
            id,
            data
        );

        link!("store.remove", store_remove, entity_ptr, id_ptr);

        link!("typeConversion.bytesToString", bytes_to_string, ptr);
        link!("typeConversion.bytesToHex", bytes_to_hex, ptr);
        link!("typeConversion.bigIntToString", big_int_to_string, ptr);
        link!("typeConversion.bigIntToHex", big_int_to_hex, ptr);
        link!("typeConversion.stringToH160", string_to_h160, ptr);
        link!("typeConversion.bytesToBase58", bytes_to_base58, ptr);

        link!("json.fromBytes", json_from_bytes, ptr);
        link!("json.try_fromBytes", json_try_from_bytes, ptr);
        link!("json.toI64", json_to_i64, ptr);
        link!("json.toU64", json_to_u64, ptr);
        link!("json.toF64", json_to_f64, ptr);
        link!("json.toBigInt", json_to_big_int, ptr);

        link!("crypto.keccak256", crypto_keccak_256, ptr);

        link!("bigInt.plus", big_int_plus, x_ptr, y_ptr);
        link!("bigInt.minus", big_int_minus, x_ptr, y_ptr);
        link!("bigInt.times", big_int_times, x_ptr, y_ptr);
        link!("bigInt.dividedBy", big_int_divided_by, x_ptr, y_ptr);
        link!("bigInt.dividedByDecimal", big_int_divided_by_decimal, x, y);
        link!("bigInt.mod", big_int_mod, x_ptr, y_ptr);
        link!("bigInt.pow", big_int_pow, x_ptr, exp);
        link!("bigInt.fromString", big_int_from_string, ptr);
        link!("bigInt.bitOr", big_int_bit_or, x_ptr, y_ptr);
        link!("bigInt.bitAnd", big_int_bit_and, x_ptr, y_ptr);
        link!("bigInt.leftShift", big_int_left_shift, x_ptr, bits);
        link!("bigInt.rightShift", big_int_right_shift, x_ptr, bits);

        link!("bigDecimal.toString", big_decimal_to_string, ptr);
        link!("bigDecimal.fromString", big_decimal_from_string, ptr);
        link!("bigDecimal.plus", big_decimal_plus, x_ptr, y_ptr);
        link!("bigDecimal.minus", big_decimal_minus, x_ptr, y_ptr);
        link!("bigDecimal.times", big_decimal_times, x_ptr, y_ptr);
        link!("bigDecimal.dividedBy", big_decimal_divided_by, x, y);
        link!("bigDecimal.equals", big_decimal_equals, x_ptr, y_ptr);

        link!("dataSource.create", data_source_create, name, params);
        link!(
            "dataSource.createWithContext",
            data_source_create_with_context,
            name,
            params,
            context
        );
        link!("dataSource.address", data_source_address,);
        link!("dataSource.network", data_source_network,);
        link!("dataSource.context", data_source_context,);

        link!("log.log", log_log, level, msg_ptr);
        let instance = linker.instantiate(&valid_module.module)?;
        // Usually `shared_ctx` is still `None` because no host fns were called during start.
        let mut shared_ctx_val = shared_ctx.lock().unwrap();
        if shared_ctx_val.is_none() {
            println!("Create WasmInstanceContext from instance");
            let ctx_val = ctx.lock().unwrap();
            *shared_ctx_val = Some(WasmInstanceContext::from_instance(
                &instance,
                *ctx_val.unwrap(),
                valid_module,
                host_metrics,
                timeout,
                timeout_stopwatch,
                //experimental_features,
            )?);
        }

        match api_version {
            version if version <= Version::new(0, 0, 4) => {}
            _ => {
                instance
                    .get_func("_start")
                    .context("`_start` function not found")?
                    .typed::<(), ()>()?
                    .call(())
                    .unwrap();
            }
        }
        Ok(WasmInstance {
            instance: Arc::new(instance),
            instance_ctx: shared_ctx,
        })
    }
}
*/
