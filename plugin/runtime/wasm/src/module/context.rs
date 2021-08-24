use crate::asc_abi::class::*;
use crate::graph::prelude::{BigDecimal, BigInt};
use crate::graph::{
    cheap_clone::CheapClone,
    runtime::{
        asc_get, asc_new, try_asc_get, AscHeap, AscPtr, DeterministicHostError, HostExportError,
        IndexForAscTypeId,
    },
    HostMetrics,
};
use crate::host_exports;
use crate::indexer::blockchain::Blockchain;
use crate::mapping::{MappingContext, ValidModule};
use crate::module::TimeoutStopwatch;
use crate::prelude::{warn, Arc, Version};
use crate::store;
use massbit_common::prelude::anyhow::{anyhow, Context};
use never::Never;
use std::rc::Rc;
use std::{
    collections::HashMap,
    convert::TryFrom,
    marker::{Send, Sync},
    time::{Duration, Instant},
};
use wasmtime::Memory;

pub struct WasmInstanceContext<C: Blockchain> {
    // In the future there may be multiple memories, but currently there is only one memory per
    // module. And at least AS calls it "memory". There is no uninitialized memory in Wasm, memory
    // is zeroed when initialized or grown.
    memory: Memory,

    // Function exported by the wasm module that will allocate the request number of bytes and
    // return a pointer to the first byte of allocated space.
    memory_allocate: wasmtime::TypedFunc<i32, i32>,

    // Function wrapper for `idof<T>` from AssemblyScript
    id_of_type: Option<wasmtime::TypedFunc<u32, u32>>,

    pub ctx: MappingContext<C>,
    pub valid_module: Arc<ValidModule>,
    pub host_metrics: Arc<HostMetrics>,
    pub(crate) timeout: Option<Duration>,

    // Used by ipfs.map.
    pub(crate) timeout_stopwatch: Arc<std::sync::Mutex<TimeoutStopwatch>>,

    // First free byte in the current arena. Set on the first call to `raw_new`.
    arena_start_ptr: i32,

    // Number of free bytes starting from `arena_start_ptr`.
    arena_free_size: i32,

    // A trap ocurred due to a possible reorg detection.
    pub possible_reorg: bool,

    // A host export trap ocurred for a deterministic reason.
    pub deterministic_host_trap: bool,
    //pub(crate) experimental_features: ExperimentalFeatures,
}

impl<C: Blockchain> AscHeap for WasmInstanceContext<C> {
    fn raw_new(&mut self, bytes: &[u8]) -> Result<u32, DeterministicHostError> {
        // We request large chunks from the AssemblyScript allocator to use as arenas that we
        // manage directly.

        static MIN_ARENA_SIZE: i32 = 10_000;

        let size = i32::try_from(bytes.len()).unwrap();
        if size > self.arena_free_size {
            // Allocate a new arena. Any free space left in the previous arena is left unused. This
            // causes at most half of memory to be wasted, which is acceptable.
            let arena_size = size.max(MIN_ARENA_SIZE);

            // Unwrap: This may panic if more memory needs to be requested from the OS and that
            // fails. This error is not deterministic since it depends on the operating conditions
            // of the node.
            self.arena_start_ptr = self.memory_allocate.call(arena_size).unwrap();
            self.arena_free_size = arena_size;

            match &self.ctx.host_exports.api_version {
                version if *version <= Version::new(0, 0, 4) => {}
                _ => {
                    // This arithmetic is done because when you call AssemblyScripts's `__alloc`
                    // function, it isn't typed and it just returns `mmInfo` on it's header,
                    // differently from allocating on regular types (`__new` for example).
                    // `mmInfo` has size of 4, and everything allocated on AssemblyScript memory
                    // should have alignment of 16, this means we need to do a 12 offset on these
                    // big chunks of untyped allocation.
                    self.arena_start_ptr += 12;
                    self.arena_free_size -= 12;
                }
            };
        };

        let ptr = self.arena_start_ptr as usize;

        // Unwrap: We have just allocated enough space for `bytes`.
        self.memory.write(ptr, bytes).unwrap();
        self.arena_start_ptr += size;
        self.arena_free_size -= size;

        Ok(ptr as u32)
    }

    fn get(&self, offset: u32, size: u32) -> Result<Vec<u8>, DeterministicHostError> {
        let offset = offset as usize;
        let size = size as usize;

        let mut data = vec![0; size];

        self.memory.read(offset, &mut data).map_err(|_| {
            DeterministicHostError(anyhow!(
                "Heap access out of bounds. Offset: {} Size: {}",
                offset,
                size
            ))
        })?;

        Ok(data)
    }

    fn api_version(&self) -> Version {
        self.ctx.host_exports.api_version.clone()
    }

    fn asc_type_id(
        &mut self,
        type_id_index: IndexForAscTypeId,
    ) -> Result<u32, DeterministicHostError> {
        let type_id = self
            .id_of_type
            .as_ref()
            .unwrap() // Unwrap ok because it's only called on correct apiVersion, look for AscPtr::generate_header
            .call(type_id_index as u32)
            .with_context(|| format!("Failed to call 'asc_type_id' with '{:?}'", type_id_index))
            .map_err(DeterministicHostError)?;
        Ok(type_id)
    }
}

impl<C: Blockchain> WasmInstanceContext<C> {
    pub fn from_instance(
        instance: &wasmtime::Instance,
        ctx: MappingContext<C>,
        valid_module: Arc<ValidModule>,
        host_metrics: Arc<HostMetrics>,
        timeout: Option<Duration>,
        timeout_stopwatch: Arc<std::sync::Mutex<TimeoutStopwatch>>,
        //experimental_features: ExperimentalFeatures,
    ) -> Result<Self, anyhow::Error> {
        // Provide access to the WASM runtime linear memory
        let memory = instance
            .get_memory("memory")
            .context("Failed to find memory export in the WASM module")?;

        let memory_allocate = match &ctx.host_exports.api_version {
            version if *version <= Version::new(0, 0, 4) => instance
                .get_func("memory.allocate")
                .context("`memory.allocate` function not found"),
            _ => instance
                .get_func("allocate")
                .context("`allocate` function not found"),
        }?
        .typed()?
        .clone();

        let id_of_type = match &ctx.host_exports.api_version {
            version if *version <= Version::new(0, 0, 4) => None,
            _ => Some(
                instance
                    .get_func("id_of_type")
                    .context("`id_of_type` function not found")?
                    .typed()?
                    .clone(),
            ),
        };

        Ok(WasmInstanceContext {
            memory_allocate,
            id_of_type,
            memory,
            ctx,
            valid_module,
            host_metrics,
            timeout,
            timeout_stopwatch,
            arena_free_size: 0,
            arena_start_ptr: 0,
            possible_reorg: false,
            deterministic_host_trap: false,
            //experimental_features,
        })
    }

    pub fn from_caller(
        caller: wasmtime::Caller,
        ctx: MappingContext<C>,
        valid_module: Arc<ValidModule>,
        host_metrics: Arc<HostMetrics>,
        timeout: Option<Duration>,
        timeout_stopwatch: Arc<std::sync::Mutex<TimeoutStopwatch>>,
        //experimental_features: ExperimentalFeatures,
    ) -> Result<Self, anyhow::Error> {
        let memory = caller
            .get_export("memory")
            .and_then(|e| e.into_memory())
            .context("Failed to find memory export in the WASM module")?;

        let memory_allocate = match &ctx.host_exports.api_version {
            version if *version <= Version::new(0, 0, 4) => caller
                .get_export("memory.allocate")
                .and_then(|e| e.into_func())
                .context("`memory.allocate` function not found"),
            _ => caller
                .get_export("allocate")
                .and_then(|e| e.into_func())
                .context("`allocate` function not found"),
        }?
        .typed()?
        .clone();

        let id_of_type = match &ctx.host_exports.api_version {
            version if *version <= Version::new(0, 0, 4) => None,
            _ => Some(
                caller
                    .get_export("id_of_type")
                    .and_then(|e| e.into_func())
                    .context("`id_of_type` function not found")?
                    .typed()?
                    .clone(),
            ),
        };

        Ok(WasmInstanceContext {
            id_of_type,
            memory_allocate,
            memory,
            ctx,
            valid_module,
            host_metrics,
            timeout,
            timeout_stopwatch,
            arena_free_size: 0,
            arena_start_ptr: 0,
            possible_reorg: false,
            deterministic_host_trap: false,
            //experimental_features,
        })
    }
}

// Implementation of externals.
impl<C: Blockchain> WasmInstanceContext<C> {
    /// function abort(message?: string | null, fileName?: string | null, lineNumber?: u32, columnNumber?: u32): void
    /// Always returns a trap.
    pub fn abort(
        &mut self,
        message_ptr: AscPtr<AscString>,
        file_name_ptr: AscPtr<AscString>,
        line_number: u32,
        column_number: u32,
    ) -> Result<Never, DeterministicHostError> {
        let message = match message_ptr.is_null() {
            false => Some(asc_get(self, message_ptr)?),
            true => None,
        };
        let file_name = match file_name_ptr.is_null() {
            false => Some(asc_get(self, file_name_ptr)?),
            true => None,
        };
        let line_number = match line_number {
            0 => None,
            _ => Some(line_number),
        };
        let column_number = match column_number {
            0 => None,
            _ => Some(column_number),
        };

        self.ctx
            .host_exports
            .abort(message, file_name, line_number, column_number)
    }

    /// function store.get(entity: string, id: string): Entity | null
    pub fn store_get(
        &mut self,
        entity_ptr: AscPtr<AscString>,
        id_ptr: AscPtr<AscString>,
    ) -> Result<AscPtr<AscEntity>, HostExportError> {
        let _timer = self
            .host_metrics
            .cheap_clone()
            .time_host_fn_execution_region("store_get");
        let entity_ptr = asc_get(self, entity_ptr)?;
        let id_ptr = asc_get(self, id_ptr)?;
        let entity_option =
            self.ctx
                .host_exports
                .store_get(&mut self.ctx.state, entity_ptr, id_ptr)?;
        let ret = match entity_option {
            Some(entity) => {
                let _section = self
                    .host_metrics
                    .stopwatch
                    .start_section("store_get_asc_new");
                asc_new(self, &entity.sorted())?
            }
            None => AscPtr::null(),
        };

        Ok(ret)
    }

    /// function store.set(entity: string, id: string, data: Entity): void
    pub fn store_set(
        &mut self,
        entity_ptr: AscPtr<AscString>,
        id_ptr: AscPtr<AscString>,
        data_ptr: AscPtr<AscEntity>,
    ) -> Result<(), HostExportError> {
        let stopwatch = &self.host_metrics.stopwatch;
        stopwatch.start_section("host_export_store_set__wasm_instance_context_store_set");

        let entity = asc_get(self, entity_ptr)?;
        let id = asc_get(self, id_ptr)?;
        let data = try_asc_get(self, data_ptr)?;

        self.ctx.host_exports.store_set(
            &self.ctx.logger,
            &mut self.ctx.state,
            //&self.ctx.proof_of_indexing,
            entity,
            id,
            data,
            stopwatch,
        )?;
        Ok(())
    }

    /// function store.remove(entity: string, id: string): void
    pub fn store_remove(
        &mut self,
        entity_ptr: AscPtr<AscString>,
        id_ptr: AscPtr<AscString>,
    ) -> Result<(), HostExportError> {
        let entity = asc_get(self, entity_ptr)?;
        let id = asc_get(self, id_ptr)?;
        self.ctx.host_exports.store_remove(
            &self.ctx.logger,
            &mut self.ctx.state,
            //&self.ctx.proof_of_indexing,
            entity,
            id,
        )
    }

    /// function typeConversion.bytesToString(bytes: Bytes): string
    pub fn bytes_to_string(
        &mut self,
        bytes_ptr: AscPtr<Uint8Array>,
    ) -> Result<AscPtr<AscString>, DeterministicHostError> {
        let string = host_exports::bytes_to_string(&self.ctx.logger, asc_get(self, bytes_ptr)?);
        asc_new(self, &string)
    }

    /// Converts bytes to a hex string.
    /// function typeConversion.bytesToHex(bytes: Bytes): string
    /// References:
    /// https://godoc.org/github.com/ethereum/go-ethereum/common/hexutil#hdr-Encoding_Rules
    /// https://github.com/ethereum/web3.js/blob/f98fe1462625a6c865125fecc9cb6b414f0a5e83/packages/web3-utils/src/utils.js#L283
    pub fn bytes_to_hex(
        &mut self,
        bytes_ptr: AscPtr<Uint8Array>,
    ) -> Result<AscPtr<AscString>, DeterministicHostError> {
        let bytes: Vec<u8> = asc_get(self, bytes_ptr)?;
        // Even an empty string must be prefixed with `0x`.
        // Encodes each byte as a two hex digits.
        let hex = format!("0x{}", hex::encode(bytes));
        asc_new(self, &hex)
    }

    /// function typeConversion.bigIntToString(n: Uint8Array): string
    pub fn big_int_to_string(
        &mut self,
        big_int_ptr: AscPtr<AscBigInt>,
    ) -> Result<AscPtr<AscString>, DeterministicHostError> {
        let n: BigInt = asc_get(self, big_int_ptr)?;
        asc_new(self, &n.to_string())
    }

    /// function bigInt.fromString(x: string): BigInt
    pub fn big_int_from_string(
        &mut self,
        string_ptr: AscPtr<AscString>,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_int_from_string(asc_get(self, string_ptr)?)?;
        asc_new(self, &result)
    }

    /// function typeConversion.bigIntToHex(n: Uint8Array): string
    pub fn big_int_to_hex(
        &mut self,
        big_int_ptr: AscPtr<AscBigInt>,
    ) -> Result<AscPtr<AscString>, DeterministicHostError> {
        let n: BigInt = asc_get(self, big_int_ptr)?;
        let hex = self.ctx.host_exports.big_int_to_hex(n)?;
        asc_new(self, &hex)
    }

    /// function typeConversion.stringToH160(s: String): H160
    pub fn string_to_h160(
        &mut self,
        str_ptr: AscPtr<AscString>,
    ) -> Result<AscPtr<AscH160>, DeterministicHostError> {
        let s: String = asc_get(self, str_ptr)?;
        let h160 = host_exports::string_to_h160(&s)?;
        asc_new(self, &h160)
    }

    /// function json.fromBytes(bytes: Bytes): JSONValue
    pub fn json_from_bytes(
        &mut self,
        bytes_ptr: AscPtr<Uint8Array>,
    ) -> Result<AscPtr<AscEnum<JsonValueKind>>, DeterministicHostError> {
        let bytes: Vec<u8> = asc_get(self, bytes_ptr)?;

        let result = host_exports::json_from_bytes(&bytes)
            .with_context(|| {
                format!(
                    "Failed to parse JSON from byte array. Bytes (truncated to 1024 chars): `{:?}`",
                    &bytes[..bytes.len().min(1024)],
                )
            })
            .map_err(DeterministicHostError)?;
        asc_new(self, &result)
    }

    /// function json.try_fromBytes(bytes: Bytes): Result<JSONValue, boolean>
    pub fn json_try_from_bytes(
        &mut self,
        bytes_ptr: AscPtr<Uint8Array>,
    ) -> Result<AscPtr<AscResult<AscPtr<AscEnum<JsonValueKind>>, bool>>, DeterministicHostError>
    {
        let bytes: Vec<u8> = asc_get(self, bytes_ptr)?;
        let result = host_exports::json_from_bytes(&bytes).map_err(|e| {
            warn!(
                &self.ctx.logger,
                "Failed to parse JSON from byte array";
                "bytes" => format!("{:?}", bytes),
                "error" => format!("{}", e)
            );

            // Map JSON errors to boolean to match the `Result<JSONValue, boolean>`
            // result type expected by mappings
            true
        });
        asc_new(self, &result)
    }
    /*
    /// function ipfs.cat(link: String): Bytes
    pub fn ipfs_cat(
        &mut self,
        link_ptr: AscPtr<AscString>,
    ) -> Result<AscPtr<Uint8Array>, HostExportError> {
        if !self.experimental_features.allow_non_deterministic_ipfs {
            return Err(HostExportError::Deterministic(anyhow!(
                "`ipfs.cat` is deprecated. Improved support for IPFS will be added in the future"
            )));
        }

        let link = asc_get(self, link_ptr)?;
        let ipfs_res = self.ctx.host_exports.ipfs_cat(&self.ctx.logger, link);
        match ipfs_res {
            Ok(bytes) => asc_new(self, &*bytes).map_err(Into::into),

            // Return null in case of error.
            Err(e) => {
                info!(&self.ctx.logger, "Failed ipfs.cat, returning `null`";
                                    "link" => asc_get::<String, _, _>(self, link_ptr)?,
                                    "error" => e.to_string());
                Ok(AscPtr::null())
            }
        }
    }

    /// function ipfs.map(link: String, callback: String, flags: String[]): void
    pub fn ipfs_map(
        &mut self,
        link_ptr: AscPtr<AscString>,
        callback: AscPtr<AscString>,
        user_data: AscPtr<AscEnum<StoreValueKind>>,
        flags: AscPtr<Array<AscPtr<AscString>>>,
    ) -> Result<(), HostExportError> {
        if !self.experimental_features.allow_non_deterministic_ipfs {
            return Err(HostExportError::Deterministic(anyhow!(
                "`ipfs.map` is deprecated. Improved support for IPFS will be added in the future"
            )));
        }

        let link: String = asc_get(self, link_ptr)?;
        let callback: String = asc_get(self, callback)?;
        let user_data: store::Value = try_asc_get(self, user_data)?;

        let flags = asc_get(self, flags)?;

        // Pause the timeout while running ipfs_map, ensure it will be restarted by using a guard.
        self.timeout_stopwatch.lock().unwrap().stop();
        let defer_stopwatch = self.timeout_stopwatch.clone();
        let _stopwatch_guard = defer::defer(|| defer_stopwatch.lock().unwrap().start());

        let start_time = Instant::now();
        let output_states = HostExports::ipfs_map(
            &self.ctx.host_exports.link_resolver.clone(),
            self,
            link.clone(),
            &*callback,
            user_data,
            flags,
        )?;

        debug!(
            &self.ctx.logger,
            "Successfully processed file with ipfs.map";
            "link" => &link,
            "callback" => &*callback,
            "n_calls" => output_states.len(),
            "time" => format!("{}ms", start_time.elapsed().as_millis())
        );
        for output_state in output_states {
            self.ctx.state.extend(output_state);
        }

        Ok(())
    }
    */
    /// Expects a decimal string.
    /// function json.toI64(json: String): i64
    pub fn json_to_i64(
        &mut self,
        json_ptr: AscPtr<AscString>,
    ) -> Result<i64, DeterministicHostError> {
        self.ctx.host_exports.json_to_i64(asc_get(self, json_ptr)?)
    }

    /// Expects a decimal string.
    /// function json.toU64(json: String): u64
    pub fn json_to_u64(
        &mut self,
        json_ptr: AscPtr<AscString>,
    ) -> Result<u64, DeterministicHostError> {
        self.ctx.host_exports.json_to_u64(asc_get(self, json_ptr)?)
    }

    /// Expects a decimal string.
    /// function json.toF64(json: String): f64
    pub fn json_to_f64(
        &mut self,
        json_ptr: AscPtr<AscString>,
    ) -> Result<f64, DeterministicHostError> {
        self.ctx.host_exports.json_to_f64(asc_get(self, json_ptr)?)
    }

    /// Expects a decimal string.
    /// function json.toBigInt(json: String): BigInt
    pub fn json_to_big_int(
        &mut self,
        json_ptr: AscPtr<AscString>,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let big_int = self
            .ctx
            .host_exports
            .json_to_big_int(asc_get(self, json_ptr)?)?;
        asc_new(self, &*big_int)
    }

    /// function crypto.keccak256(input: Bytes): Bytes
    pub fn crypto_keccak_256(
        &mut self,
        input_ptr: AscPtr<Uint8Array>,
    ) -> Result<AscPtr<Uint8Array>, DeterministicHostError> {
        let input = self
            .ctx
            .host_exports
            .crypto_keccak_256(asc_get(self, input_ptr)?)?;
        asc_new(self, input.as_ref())
    }

    /// function bigInt.plus(x: BigInt, y: BigInt): BigInt
    pub fn big_int_plus(
        &mut self,
        x_ptr: AscPtr<AscBigInt>,
        y_ptr: AscPtr<AscBigInt>,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_int_plus(asc_get(self, x_ptr)?, asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigInt.minus(x: BigInt, y: BigInt): BigInt
    pub fn big_int_minus(
        &mut self,
        x_ptr: AscPtr<AscBigInt>,
        y_ptr: AscPtr<AscBigInt>,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_int_minus(asc_get(self, x_ptr)?, asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigInt.times(x: BigInt, y: BigInt): BigInt
    pub fn big_int_times(
        &mut self,
        x_ptr: AscPtr<AscBigInt>,
        y_ptr: AscPtr<AscBigInt>,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_int_times(asc_get(self, x_ptr)?, asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigInt.dividedBy(x: BigInt, y: BigInt): BigInt
    pub fn big_int_divided_by(
        &mut self,
        x_ptr: AscPtr<AscBigInt>,
        y_ptr: AscPtr<AscBigInt>,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_int_divided_by(asc_get(self, x_ptr)?, asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigInt.dividedByDecimal(x: BigInt, y: BigDecimal): BigDecimal
    pub fn big_int_divided_by_decimal(
        &mut self,
        x_ptr: AscPtr<AscBigInt>,
        y_ptr: AscPtr<AscBigDecimal>,
    ) -> Result<AscPtr<AscBigDecimal>, DeterministicHostError> {
        let x = BigDecimal::new(asc_get::<BigInt, _, _>(self, x_ptr)?, 0);
        let result = self
            .ctx
            .host_exports
            .big_decimal_divided_by(x, try_asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigInt.mod(x: BigInt, y: BigInt): BigInt
    pub fn big_int_mod(
        &mut self,
        x_ptr: AscPtr<AscBigInt>,
        y_ptr: AscPtr<AscBigInt>,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_int_mod(asc_get(self, x_ptr)?, asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigInt.pow(x: BigInt, exp: u8): BigInt
    pub fn big_int_pow(
        &mut self,
        x_ptr: AscPtr<AscBigInt>,
        exp: u32,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let exp = u8::try_from(exp).map_err(|e| DeterministicHostError(e.into()))?;
        let result = self
            .ctx
            .host_exports
            .big_int_pow(asc_get(self, x_ptr)?, exp)?;
        asc_new(self, &result)
    }

    /// function bigInt.bitOr(x: BigInt, y: BigInt): BigInt
    pub fn big_int_bit_or(
        &mut self,
        x_ptr: AscPtr<AscBigInt>,
        y_ptr: AscPtr<AscBigInt>,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_int_bit_or(asc_get(self, x_ptr)?, asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigInt.bitAnd(x: BigInt, y: BigInt): BigInt
    pub fn big_int_bit_and(
        &mut self,
        x_ptr: AscPtr<AscBigInt>,
        y_ptr: AscPtr<AscBigInt>,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_int_bit_and(asc_get(self, x_ptr)?, asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigInt.leftShift(x: BigInt, bits: u8): BigInt
    pub fn big_int_left_shift(
        &mut self,
        x_ptr: AscPtr<AscBigInt>,
        bits: u32,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let bits = u8::try_from(bits).map_err(|e| DeterministicHostError(e.into()))?;
        let result = self
            .ctx
            .host_exports
            .big_int_left_shift(asc_get(self, x_ptr)?, bits)?;
        asc_new(self, &result)
    }

    /// function bigInt.rightShift(x: BigInt, bits: u8): BigInt
    pub fn big_int_right_shift(
        &mut self,
        x_ptr: AscPtr<AscBigInt>,
        bits: u32,
    ) -> Result<AscPtr<AscBigInt>, DeterministicHostError> {
        let bits = u8::try_from(bits).map_err(|e| DeterministicHostError(e.into()))?;
        let result = self
            .ctx
            .host_exports
            .big_int_right_shift(asc_get(self, x_ptr)?, bits)?;
        asc_new(self, &result)
    }

    /// function typeConversion.bytesToBase58(bytes: Bytes): string
    pub fn bytes_to_base58(
        &mut self,
        bytes_ptr: AscPtr<Uint8Array>,
    ) -> Result<AscPtr<AscString>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .bytes_to_base58(asc_get(self, bytes_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigDecimal.toString(x: BigDecimal): string
    pub fn big_decimal_to_string(
        &mut self,
        big_decimal_ptr: AscPtr<AscBigDecimal>,
    ) -> Result<AscPtr<AscString>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_decimal_to_string(try_asc_get(self, big_decimal_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigDecimal.fromString(x: string): BigDecimal
    pub fn big_decimal_from_string(
        &mut self,
        string_ptr: AscPtr<AscString>,
    ) -> Result<AscPtr<AscBigDecimal>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_decimal_from_string(asc_get(self, string_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigDecimal.plus(x: BigDecimal, y: BigDecimal): BigDecimal
    pub fn big_decimal_plus(
        &mut self,
        x_ptr: AscPtr<AscBigDecimal>,
        y_ptr: AscPtr<AscBigDecimal>,
    ) -> Result<AscPtr<AscBigDecimal>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_decimal_plus(try_asc_get(self, x_ptr)?, try_asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigDecimal.minus(x: BigDecimal, y: BigDecimal): BigDecimal
    pub fn big_decimal_minus(
        &mut self,
        x_ptr: AscPtr<AscBigDecimal>,
        y_ptr: AscPtr<AscBigDecimal>,
    ) -> Result<AscPtr<AscBigDecimal>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_decimal_minus(try_asc_get(self, x_ptr)?, try_asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigDecimal.times(x: BigDecimal, y: BigDecimal): BigDecimal
    pub fn big_decimal_times(
        &mut self,
        x_ptr: AscPtr<AscBigDecimal>,
        y_ptr: AscPtr<AscBigDecimal>,
    ) -> Result<AscPtr<AscBigDecimal>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_decimal_times(try_asc_get(self, x_ptr)?, try_asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigDecimal.dividedBy(x: BigDecimal, y: BigDecimal): BigDecimal
    pub fn big_decimal_divided_by(
        &mut self,
        x_ptr: AscPtr<AscBigDecimal>,
        y_ptr: AscPtr<AscBigDecimal>,
    ) -> Result<AscPtr<AscBigDecimal>, DeterministicHostError> {
        let result = self
            .ctx
            .host_exports
            .big_decimal_divided_by(try_asc_get(self, x_ptr)?, try_asc_get(self, y_ptr)?)?;
        asc_new(self, &result)
    }

    /// function bigDecimal.equals(x: BigDecimal, y: BigDecimal): bool
    pub fn big_decimal_equals(
        &mut self,
        x_ptr: AscPtr<AscBigDecimal>,
        y_ptr: AscPtr<AscBigDecimal>,
    ) -> Result<bool, DeterministicHostError> {
        self.ctx
            .host_exports
            .big_decimal_equals(try_asc_get(self, x_ptr)?, try_asc_get(self, y_ptr)?)
    }

    /// function dataSource.create(name: string, params: Array<string>): void
    pub fn data_source_create(
        &mut self,
        name_ptr: AscPtr<AscString>,
        params_ptr: AscPtr<Array<AscPtr<AscString>>>,
    ) -> Result<(), HostExportError> {
        let name: String = asc_get(self, name_ptr)?;
        let params: Vec<String> = asc_get(self, params_ptr)?;
        self.ctx.host_exports.data_source_create(
            &self.ctx.logger,
            &mut self.ctx.state,
            name,
            params,
            None,
            self.ctx.block_ptr.number,
        )
    }

    /// function createWithContext(name: string, params: Array<string>, context: DataSourceContext): void
    pub fn data_source_create_with_context(
        &mut self,
        name_ptr: AscPtr<AscString>,
        params_ptr: AscPtr<Array<AscPtr<AscString>>>,
        context_ptr: AscPtr<AscEntity>,
    ) -> Result<(), HostExportError> {
        let name: String = asc_get(self, name_ptr)?;
        let params: Vec<String> = asc_get(self, params_ptr)?;
        let context: HashMap<_, _> = try_asc_get(self, context_ptr)?;
        self.ctx.host_exports.data_source_create(
            &self.ctx.logger,
            &mut self.ctx.state,
            name,
            params,
            Some(context.into()),
            self.ctx.block_ptr.number,
        )
    }

    /// function dataSource.address(): Bytes
    pub fn data_source_address(&mut self) -> Result<AscPtr<Uint8Array>, DeterministicHostError> {
        asc_new(self, self.ctx.host_exports.data_source_address().as_slice())
    }

    /// function dataSource.network(): String
    pub fn data_source_network(&mut self) -> Result<AscPtr<AscString>, DeterministicHostError> {
        asc_new(self, &self.ctx.host_exports.data_source_network())
    }

    /// function dataSource.context(): DataSourceContext
    pub fn data_source_context(&mut self) -> Result<AscPtr<AscEntity>, DeterministicHostError> {
        asc_new(self, &self.ctx.host_exports.data_source_context().sorted())
    }

    pub fn ens_name_by_hash(
        &mut self,
        hash_ptr: AscPtr<AscString>,
    ) -> Result<AscPtr<AscString>, HostExportError> {
        let hash: String = asc_get(self, hash_ptr)?;
        let name = self.ctx.host_exports.ens_name_by_hash(&*hash)?;
        // map `None` to `null`, and `Some(s)` to a runtime string
        name.map(|name| asc_new(self, &*name).map_err(Into::into))
            .unwrap_or(Ok(AscPtr::null()))
    }

    pub fn log_log(
        &mut self,
        level: u32,
        msg: AscPtr<AscString>,
    ) -> Result<(), DeterministicHostError> {
        let level = LogLevel::from(level).into();
        let msg: String = asc_get(self, msg)?;
        self.ctx.host_exports.log_log(&self.ctx.logger, level, msg)
    }
    /*
    ///function call(call: SmartContractCall): Array<Value> | null
    pub fn ethereum_call(
        &mut self,
        contract_ptr: AscPtr<AscEnum<SmartContractCall>>,
    ) -> Result<AscPtr<Uint8Array>, DeterministicHostError> {
        let data = host_exports::ethereum_encode(asc_get(self, contract_ptr)?);
        // return `null` if it fails
        data.map(|bytes| asc_new(self, &*bytes))
            .unwrap_or(Ok(AscPtr::null()))
    }
     */
    /// function encode(token: ethereum.Value): Bytes | null
    pub fn ethereum_encode(
        &mut self,
        token_ptr: AscPtr<AscEnum<EthereumValueKind>>,
    ) -> Result<AscPtr<Uint8Array>, DeterministicHostError> {
        let data = host_exports::ethereum_encode(asc_get(self, token_ptr)?);
        // return `null` if it fails
        data.map(|bytes| asc_new(self, &*bytes))
            .unwrap_or(Ok(AscPtr::null()))
    }

    /// function decode(types: String, data: Bytes): ethereum.Value | null
    pub fn ethereum_decode(
        &mut self,
        types_ptr: AscPtr<AscString>,
        data_ptr: AscPtr<Uint8Array>,
    ) -> Result<AscPtr<AscEnum<EthereumValueKind>>, DeterministicHostError> {
        let result =
            host_exports::ethereum_decode(asc_get(self, types_ptr)?, asc_get(self, data_ptr)?);
        // return `null` if it fails
        result
            .map(|param| asc_new(self, &param))
            .unwrap_or(Ok(AscPtr::null()))
    }

    /// function arweave.transactionData(txId: string): Bytes | null
    pub fn arweave_transaction_data(
        &mut self,
        _tx_id: AscPtr<AscString>,
    ) -> Result<AscPtr<Uint8Array>, HostExportError> {
        Err(HostExportError::Deterministic(anyhow!(
            "`arweave.transactionData` has been removed."
        )))
    }

    /// function box.profile(address: string): JSONValue | null
    pub fn box_profile(
        &mut self,
        _address: AscPtr<AscString>,
    ) -> Result<AscPtr<AscJson>, HostExportError> {
        Err(HostExportError::Deterministic(anyhow!(
            "`box.profile` has been removed."
        )))
    }
}
