use std::collections::HashMap;
use std::str::FromStr;

use massbit_common::prelude::{
    anyhow::{anyhow, Context},
    ethabi::param_type::Reader,
    ethabi::{decode, encode, Token, Uint},
    serde_json,
};

use never::Never;
use slog::{b, info, record_static, trace};
use wasmtime::Trap;
use web3::types::H160;

//use crate::asc_abi::class::{AscEnumArray, EthereumValueKind};
//use crate::chain::ethereum::runtime::abi::{
//    AscUnresolvedContractCall, AscUnresolvedContractCall_0_0_4,
//};
use graph::prelude::{
    BigDecimal, BigInt, BlockNumber, CheapClone, DataSourceTemplateInfo, StopwatchMetrics, Value,
};
use graph::runtime::{asc_get, asc_new, AscPtr, DeterministicHostError, HostExportError};
use graph_chain_ethereum::{
    runtime::{
        abi::{AscUnresolvedContractCall, AscUnresolvedContractCall_0_0_4},
        runtime_adapter::UnresolvedContractCall,
    },
    DataSource,
};
use graph_runtime_wasm::{
    asc_abi::class::{AscEnumArray, EthereumValueKind},
    error::DeterminismLevel,
};
//use crate::chain::ethereum::EthereumContractCallError;
/*
use crate::chain::ethereum::{
    data_source::DataSource,
    runtime::runtime_adapter::UnresolvedContractCall;
 */
/*
use crate::graph::cheap_clone::CheapClone;
use crate::graph::components::metrics::stopwatch::StopwatchMetrics;
use crate::graph::runtime::{asc_get, asc_new, AscPtr, DeterministicHostError, HostExportError};
 */
use crate::prelude::{slog::warn, Arc, Logger, Version};
use graph::blockchain::{
    Blockchain, DataSource as DataSourceTrait, DataSourceTemplate, HostFn, HostFnCtx,
};
use graph::prelude::BlockPtr;
//use crate::store::scalar::{BigDecimal, BigInt};
//use crate::store::{model::BlockNumber, Entity, EntityKey, EntityType, Value};

use crate::manifest::datasource::DataSourceContext;
use graph::components::store::{EntityKey, EntityType};
use graph::components::subgraph::{BlockState, Entity};
use graph::data::subgraph::DeploymentHash;
//use graph_runtime_wasm::module::IntoTrap;
use crate::module::IntoTrap;
use std::sync::Mutex;
use std::time::Instant;

impl IntoTrap for HostExportError {
    fn determinism_level(&self) -> DeterminismLevel {
        match self {
            HostExportError::Deterministic(_) => DeterminismLevel::Deterministic,
            HostExportError::Unknown(_) => DeterminismLevel::Unimplemented,
            HostExportError::PossibleReorg(_) => DeterminismLevel::PossibleReorg,
        }
    }
    fn into_trap(self) -> Trap {
        match self {
            HostExportError::Unknown(e)
            | HostExportError::PossibleReorg(e)
            | HostExportError::Deterministic(e) => Trap::from(e),
        }
    }
}

pub struct HostExports<C: Blockchain> {
    pub indexer_hash: String,
    pub api_version: Version,
    data_source_name: String,
    data_source_address: Vec<u8>,
    data_source_network: String,
    data_source_context: Arc<Option<DataSourceContext>>,
    templates: Arc<Vec<C::DataSourceTemplate>>,
}

impl<C: Blockchain> HostExports<C> {
    pub fn new(
        indexer_hash: &str,
        data_source: &impl DataSourceTrait<C>,
        data_source_network: String,
        templates: Arc<Vec<C::DataSourceTemplate>>,
        api_version: Version,
    ) -> Self {
        Self {
            indexer_hash: String::from(indexer_hash),
            api_version: api_version.clone(),
            data_source_name: data_source.name().to_owned(),
            data_source_address: data_source.address().unwrap_or_default().to_owned(),
            data_source_network,
            data_source_context: data_source.context().cheap_clone(),
            templates,
        }
    }

    pub(crate) fn abort(
        &self,
        message: Option<String>,
        file_name: Option<String>,
        line_number: Option<u32>,
        column_number: Option<u32>,
    ) -> Result<Never, DeterministicHostError> {
        let message = message
            .map(|message| format!("message: {}", message))
            .unwrap_or_else(|| "no message".into());
        let location = match (file_name, line_number, column_number) {
            (None, None, None) => "an unknown location".into(),
            (Some(file_name), None, None) => file_name,
            (Some(file_name), Some(line_number), None) => {
                format!("{}, line {}", file_name, line_number)
            }
            (Some(file_name), Some(line_number), Some(column_number)) => format!(
                "{}, line {}, column {}",
                file_name, line_number, column_number
            ),
            _ => unreachable!(),
        };
        Err(DeterministicHostError(anyhow::anyhow!(
            "Mapping aborted, with {}",
            message
        )))
    }

    pub(crate) fn store_get(
        &self,
        state: &mut BlockState<C>,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<Entity>, anyhow::Error> {
        let store_key = EntityKey {
            subgraph_id: DeploymentHash::new("_indexer").unwrap(),
            entity_type: EntityType::new(entity_type.clone()),
            entity_id: entity_id.clone(),
        };
        //let entity = state.get_entity(&store_key)?;
        Ok(state.entity_cache.get(&store_key)?)
    }

    pub(crate) fn store_set(
        &self,
        logger: &Logger,
        state: &mut BlockState<C>,
        //proof_of_indexing: &SharedProofOfIndexing,
        entity_type: String,
        entity_id: String,
        mut data: HashMap<String, Value>,
        stopwatch: &StopwatchMetrics,
    ) -> Result<(), anyhow::Error> {
        /*
        let poi_section = stopwatch.start_section("host_export_store_set__proof_of_indexing");
        if let Some(proof_of_indexing) = proof_of_indexing {
            let mut proof_of_indexing = proof_of_indexing.deref().borrow_mut();
            proof_of_indexing.write(
                logger,
                &self.causality_region,
                &ProofOfIndexingEvent::SetEntity {
                    entity_type: &entity_type,
                    id: &entity_id,
                    data: &data,
                },
            );
        }
        poi_section.end();
        */
        let id_insert_section = stopwatch.start_section("host_export_store_set__insert_id");
        // Automatically add an "id" value
        match data.insert("id".to_string(), Value::String(entity_id.clone())) {
            Some(ref v) if v != &Value::String(entity_id.clone()) => {
                return Err(anyhow!(
                    "Value of {} attribute 'id' conflicts with ID passed to `store.set()`: \
                     {} != {}",
                    entity_type,
                    v,
                    entity_id,
                ));
            }
            _ => (),
        }

        id_insert_section.end();
        let validation_section = stopwatch.start_section("host_export_store_set__validation");
        let key = EntityKey {
            subgraph_id: DeploymentHash::new("_indexer").unwrap(),
            entity_type: EntityType::new(entity_type),
            entity_id,
        };
        let entity = Entity::from(data);
        //Todo:: validate entity
        //let schema = self.store.input_schema(&self.subgraph_id)?;
        //let is_valid = validate_entity(&schema.document, &key, &entity).is_ok();
        //state.set_entity(key.clone(), entity);
        state.entity_cache.set(key.clone(), entity);
        validation_section.end();
        // Validate the changes against the subgraph schema.
        // If the set of fields we have is already valid, avoid hitting the DB.
        /*
        if !is_valid {
            stopwatch.start_section("host_export_store_set__post_validation");
            let entity = state
                .entity_cache
                .get(&key)
                .map_err(|e| HostExportError::Unknown(e.into()))?
                .expect("we just stored this entity");
            validate_entity(&schema.docuroment, &key, &entity)?;
        }
         */
        Ok(())
    }

    pub(crate) fn store_remove(
        &self,
        logger: &Logger,
        state: &mut BlockState<C>,
        //proof_of_indexing: &SharedProofOfIndexing,
        entity_type: String,
        entity_id: String,
    ) -> Result<(), HostExportError> {
        /*
        if let Some(proof_of_indexing) = proof_of_indexing {
            let mut proof_of_indexing = proof_of_indexing.deref().borrow_mut();
            proof_of_indexing.write(
                logger,
                &self.causality_region,
                &ProofOfIndexingEvent::RemoveEntity {
                    entity_type: &entity_type,
                    id: &entity_id,
                },
            );
        }
         */
        let key = EntityKey {
            subgraph_id: DeploymentHash::new(self.indexer_hash.to_string()).unwrap(),
            entity_type: EntityType::new(entity_type),
            entity_id,
        };
        state.entity_cache.remove(key);

        Ok(())
    }

    /// Prints the module of `n` in hex.
    /// Integers are encoded using the least amount of digits (no leading zero digits).
    /// Their encoding may be of uneven length. The number zero encodes as "0x0".
    ///
    /// https://godoc.org/github.com/ethereum/go-ethereum/common/hexutil#hdr-Encoding_Rules
    pub(crate) fn big_int_to_hex(&self, n: BigInt) -> Result<String, DeterministicHostError> {
        if n == 0.into() {
            return Ok("0x0".to_string());
        }

        let bytes = n.to_bytes_be().1;
        Ok(format!(
            "0x{}",
            ::hex::encode(bytes).trim_start_matches('0')
        ))
    }
    /// Expects a decimal string.
    pub(crate) fn json_to_i64(&self, json: String) -> Result<i64, DeterministicHostError> {
        i64::from_str(&json)
            .with_context(|| format!("JSON `{}` cannot be parsed as i64", json))
            .map_err(DeterministicHostError)
    }

    /// Expects a decimal string.
    pub(crate) fn json_to_u64(&self, json: String) -> Result<u64, DeterministicHostError> {
        u64::from_str(&json)
            .with_context(|| format!("JSON `{}` cannot be parsed as u64", json))
            .map_err(DeterministicHostError)
    }

    /// Expects a decimal string.
    pub(crate) fn json_to_f64(&self, json: String) -> Result<f64, DeterministicHostError> {
        f64::from_str(&json)
            .with_context(|| format!("JSON `{}` cannot be parsed as f64", json))
            .map_err(DeterministicHostError)
    }

    /// Expects a decimal string.
    pub(crate) fn json_to_big_int(&self, json: String) -> Result<Vec<u8>, DeterministicHostError> {
        let big_int = BigInt::from_str(&json)
            .with_context(|| format!("JSON `{}` is not a decimal string", json))
            .map_err(DeterministicHostError)?;
        Ok(big_int.to_signed_bytes_le())
    }

    pub(crate) fn crypto_keccak_256(
        &self,
        input: Vec<u8>,
    ) -> Result<[u8; 32], DeterministicHostError> {
        Ok(tiny_keccak::keccak256(&input))
    }

    pub(crate) fn big_int_plus(
        &self,
        x: BigInt,
        y: BigInt,
    ) -> Result<BigInt, DeterministicHostError> {
        Ok(x + y)
    }

    pub(crate) fn big_int_minus(
        &self,
        x: BigInt,
        y: BigInt,
    ) -> Result<BigInt, DeterministicHostError> {
        Ok(x - y)
    }

    pub(crate) fn big_int_times(
        &self,
        x: BigInt,
        y: BigInt,
    ) -> Result<BigInt, DeterministicHostError> {
        Ok(x * y)
    }

    pub(crate) fn big_int_divided_by(
        &self,
        x: BigInt,
        y: BigInt,
    ) -> Result<BigInt, DeterministicHostError> {
        if y == 0.into() {
            return Err(DeterministicHostError(anyhow::anyhow!(
                "attempted to divide BigInt `{}` by zero",
                x
            )));
        }
        Ok(x / y)
    }

    pub(crate) fn big_int_mod(
        &self,
        x: BigInt,
        y: BigInt,
    ) -> Result<BigInt, DeterministicHostError> {
        if y == 0.into() {
            return Err(DeterministicHostError(anyhow::anyhow!(
                "attempted to calculate the remainder of `{}` with a divisor of zero",
                x
            )));
        }
        Ok(x % y)
    }

    /// Limited to a small exponent to avoid creating huge BigInts.
    pub(crate) fn big_int_pow(
        &self,
        x: BigInt,
        exponent: u8,
    ) -> Result<BigInt, DeterministicHostError> {
        Ok(x.pow(exponent))
    }

    pub(crate) fn big_int_from_string(&self, s: String) -> Result<BigInt, DeterministicHostError> {
        BigInt::from_str(&s)
            .with_context(|| format!("string is not a BigInt: `{}`", s))
            .map_err(DeterministicHostError)
    }

    pub(crate) fn big_int_bit_or(
        &self,
        x: BigInt,
        y: BigInt,
    ) -> Result<BigInt, DeterministicHostError> {
        Ok(x | y)
    }

    pub(crate) fn big_int_bit_and(
        &self,
        x: BigInt,
        y: BigInt,
    ) -> Result<BigInt, DeterministicHostError> {
        Ok(x & y)
    }

    pub(crate) fn big_int_left_shift(
        &self,
        x: BigInt,
        bits: u8,
    ) -> Result<BigInt, DeterministicHostError> {
        Ok(x << bits)
    }

    pub(crate) fn big_int_right_shift(
        &self,
        x: BigInt,
        bits: u8,
    ) -> Result<BigInt, DeterministicHostError> {
        Ok(x >> bits)
    }

    /// Useful for IPFS hashes stored as bytes
    pub(crate) fn bytes_to_base58(&self, bytes: Vec<u8>) -> Result<String, DeterministicHostError> {
        Ok(::bs58::encode(&bytes).into_string())
    }

    pub(crate) fn big_decimal_plus(
        &self,
        x: BigDecimal,
        y: BigDecimal,
    ) -> Result<BigDecimal, DeterministicHostError> {
        Ok(x + y)
    }

    pub(crate) fn big_decimal_minus(
        &self,
        x: BigDecimal,
        y: BigDecimal,
    ) -> Result<BigDecimal, DeterministicHostError> {
        Ok(x - y)
    }

    pub(crate) fn big_decimal_times(
        &self,
        x: BigDecimal,
        y: BigDecimal,
    ) -> Result<BigDecimal, DeterministicHostError> {
        Ok(x * y)
    }

    /// Maximum precision of 100 decimal digits.
    pub(crate) fn big_decimal_divided_by(
        &self,
        x: BigDecimal,
        y: BigDecimal,
    ) -> Result<BigDecimal, DeterministicHostError> {
        if y == 0.into() {
            return Err(DeterministicHostError(anyhow::anyhow!(
                "attempted to divide BigDecimal `{}` by zero",
                x
            )));
        }
        Ok(x / y)
    }

    pub(crate) fn big_decimal_equals(
        &self,
        x: BigDecimal,
        y: BigDecimal,
    ) -> Result<bool, DeterministicHostError> {
        Ok(x == y)
    }

    pub(crate) fn big_decimal_to_string(
        &self,
        x: BigDecimal,
    ) -> Result<String, DeterministicHostError> {
        Ok(x.to_string())
    }

    pub(crate) fn big_decimal_from_string(
        &self,
        s: String,
    ) -> Result<BigDecimal, DeterministicHostError> {
        BigDecimal::from_str(&s)
            .with_context(|| format!("string  is not a BigDecimal: '{}'", s))
            .map_err(DeterministicHostError)
    }
    pub(crate) fn data_source_create(
        &self,
        logger: &Logger,
        state: &mut BlockState<C>,
        name: String,
        params: Vec<String>,
        context: Option<DataSourceContext>,
        creation_block: BlockNumber,
    ) -> Result<(), HostExportError> {
        info!(
            logger,
            "Create data source";
            "name" => &name,
            "params" => format!("{}", params.join(","))
        );

        // Resolve the name into the right template
        let template = self
            .templates
            .iter()
            .find(|template| template.name() == name)
            .with_context(|| {
                format!(
                    "Failed to create data source from name `{}`: \
                     No template with this name in parent data source `{}`. \
                     Available names: {}.",
                    name,
                    self.data_source_name,
                    self.templates
                        .iter()
                        .map(|template| template.name().clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .map_err(DeterministicHostError)?
            .clone();

        // Remember that we need to create this data source
        state.push_created_data_source(DataSourceTemplateInfo {
            template,
            params,
            context,
            creation_block,
        });

        Ok(())
    }

    pub(crate) fn ens_name_by_hash(&self, hash: &str) -> Result<Option<String>, anyhow::Error> {
        //Ok(self.store.find_ens_name(hash)?)
        Ok(None)
    }

    pub(crate) fn log_log(
        &self,
        logger: &Logger,
        level: slog::Level,
        msg: String,
    ) -> Result<(), DeterministicHostError> {
        let rs = record_static!(level, self.data_source_name.as_str());

        logger.log(&slog::Record::new(
            &rs,
            &format_args!("{}", msg),
            b!("data_source" => &self.data_source_name),
        ));

        if level == slog::Level::Critical {
            return Err(DeterministicHostError(anyhow!(
                "Critical error logged in mapping"
            )));
        }
        Ok(())
    }
    pub(crate) fn data_source_address(&self) -> Vec<u8> {
        self.data_source_address.clone()
    }

    pub(crate) fn data_source_network(&self) -> String {
        self.data_source_network.clone()
    }

    pub(crate) fn data_source_context(&self) -> Entity {
        self.data_source_context
            .as_ref()
            .clone()
            .unwrap_or_default()
    }
}
pub(crate) fn ethereum_encode(token: Token) -> Result<Vec<u8>, anyhow::Error> {
    Ok(encode(&[token]))
}

pub(crate) fn ethereum_decode(types: String, data: Vec<u8>) -> Result<Token, anyhow::Error> {
    let param_types =
        Reader::read(&types).or_else(|e| Err(anyhow::anyhow!("Failed to read types: {}", e)))?;

    decode(&[param_types], &data)
        // The `.pop().unwrap()` here is ok because we're always only passing one
        // `param_types` to `decode`, so the returned `Vec` has always size of one.
        // We can't do `tokens[0]` because the value can't be moved out of the `Vec`.
        .map(|mut tokens| tokens.pop().unwrap())
        .context("Failed to decode")
}

pub(crate) fn json_from_bytes(
    bytes: &Vec<u8>,
) -> Result<serde_json::Value, DeterministicHostError> {
    serde_json::from_reader(bytes.as_slice()).map_err(|e| DeterministicHostError(e.into()))
}

pub(crate) fn string_to_h160(string: &str) -> Result<H160, DeterministicHostError> {
    // `H160::from_str` takes a hex string with no leading `0x`.
    let s = string.trim_start_matches("0x");
    H160::from_str(s)
        .with_context(|| format!("Failed to convert string to Address/H160: '{}'", s))
        .map_err(DeterministicHostError)
}

pub(crate) fn bytes_to_string(logger: &Logger, bytes: Vec<u8>) -> String {
    let s = String::from_utf8_lossy(&bytes);

    // If the string was re-allocated, that means it was not UTF8.
    if matches!(s, std::borrow::Cow::Owned(_)) {
        warn!(
            logger,
            "Bytes contain invalid UTF8. This may be caused by attempting \
            to convert a value such as an address that cannot be parsed to a unicode string. \
            You may want to use 'toHexString()' instead. String (truncated to 1024 chars): '{}'",
            &s.chars().take(1024).collect::<String>(),
        )
    }

    // The string may have been encoded in a fixed length buffer and padded with null
    // characters, so trim trailing nulls.
    s.trim_end_matches('\u{0000}').to_string()
}
//mock ethereum.call
pub fn create_mock_ethereum_call(datasource: &DataSource) -> HostFn {
    HostFn {
        name: "ethereum.call",
        func: Arc::new(move |ctx, wasm_ptr| ethereum_call(ctx, wasm_ptr).map(|ptr| ptr.wasm_ptr())),
    }
}
fn ethereum_call(
    ctx: HostFnCtx<'_>,
    wasm_ptr: u32,
    //abis: &[Arc<MappingABI>],
) -> Result<AscEnumArray<EthereumValueKind>, HostExportError> {
    let call: UnresolvedContractCall = if ctx.heap.api_version() >= Version::new(0, 0, 4) {
        asc_get::<_, AscUnresolvedContractCall_0_0_4, _>(ctx.heap, wasm_ptr.into())?
    } else {
        asc_get::<_, AscUnresolvedContractCall, _>(ctx.heap, wasm_ptr.into())?
    };
    println!("Ethereum call: {:?}", &call);
    let tokens = match call.function_name.as_str() {
        "name" => vec![Token::String("name".to_string())],
        "symbol" => vec![Token::String("F0X".to_string())],
        "totalSupply" => vec![Token::Uint(Uint::from(rand::random::<u128>()))],
        "decimals" => vec![Token::Uint(Uint::from(rand::random::<u8>()))],
        _ => vec![],
    };
    Ok(asc_new(ctx.heap, tokens.as_slice())?)
}
/*
/// function ethereum.call(call: SmartContractCall): Array<Token> | null
fn ethereum_call(
    eth_adapter: &EthereumAdapter,
    call_cache: Arc<dyn EthereumCallCache>,
    ctx: HostFnCtx<'_>,
    wasm_ptr: u32,
    abis: &[Arc<MappingABI>],
) -> Result<AscEnumArray<EthereumValueKind>, HostExportError> {
    // For apiVersion >= 0.0.4 the call passed from the mapping includes the
    // function signature; subgraphs using an apiVersion < 0.0.4 don't pass
    // the signature along with the call.
    let call: UnresolvedContractCall = if ctx.heap.api_version() >= Version::new(0, 0, 4) {
        asc_get::<_, AscUnresolvedContractCall_0_0_4, _>(ctx.heap, wasm_ptr.into())?
    } else {
        asc_get::<_, AscUnresolvedContractCall, _>(ctx.heap, wasm_ptr.into())?
    };

    let result = eth_call(
        eth_adapter,
        call_cache,
        &ctx.logger,
        &ctx.block_ptr,
        call,
        abis,
    )?;
    match result {
        Some(tokens) => Ok(asc_new(ctx.heap, tokens.as_slice())?),
        None => Ok(AscPtr::null()),
    }
}

/// Returns `Ok(None)` if the call was reverted.
fn eth_call(
    eth_adapter: &EthereumAdapter,
    call_cache: Arc<dyn EthereumCallCache>,
    logger: &Logger,
    block_ptr: &BlockPtr,
    unresolved_call: UnresolvedContractCall,
    abis: &[Arc<MappingABI>],
) -> Result<Option<Vec<Token>>, HostExportError> {
    let start_time = Instant::now();

    // Obtain the path to the contract ABI
    let contract = abis
        .iter()
        .find(|abi| abi.name == unresolved_call.contract_name)
        .with_context(|| {
            format!(
                "Could not find ABI for contract \"{}\", try adding it to the 'abis' section \
                     of the subgraph manifest",
                unresolved_call.contract_name
            )
        })?
        .contract
        .clone();

    let function = match unresolved_call.function_signature {
        // Behavior for apiVersion < 0.0.4: look up function by name; for overloaded
        // functions this always picks the same overloaded variant, which is incorrect
        // and may lead to encoding/decoding errors
        None => contract
            .function(unresolved_call.function_name.as_str())
            .with_context(|| {
                format!(
                    "Unknown function \"{}::{}\" called from WASM runtime",
                    unresolved_call.contract_name, unresolved_call.function_name
                )
            })?,

        // Behavior for apiVersion >= 0.0.04: look up function by signature of
        // the form `functionName(uint256,string) returns (bytes32,string)`; this
        // correctly picks the correct variant of an overloaded function
        Some(ref function_signature) => contract
            .functions_by_name(unresolved_call.function_name.as_str())
            .with_context(|| {
                format!(
                    "Unknown function \"{}::{}\" called from WASM runtime",
                    unresolved_call.contract_name, unresolved_call.function_name
                )
            })?
            .iter()
            .find(|f| function_signature == &f.signature())
            .with_context(|| {
                format!(
                    "Unknown function \"{}::{}\" with signature `{}` \
                         called from WASM runtime",
                    unresolved_call.contract_name,
                    unresolved_call.function_name,
                    function_signature,
                )
            })?,
    };

    let call = EthereumContractCall {
        address: unresolved_call.contract_address.clone(),
        block_ptr: block_ptr.cheap_clone(),
        function: function.clone(),
        args: unresolved_call.function_args.clone(),
    };

    // Run Ethereum call in tokio runtime
    let logger1 = logger.clone();
    let call_cache = call_cache.clone();
    let result = match crate::graph::block_on(
        eth_adapter.contract_call(&logger1, call, call_cache).compat()
    ) {
        Ok(tokens) => Ok(Some(tokens)),
        Err(EthereumContractCallError::Revert(reason)) => {
            info!(logger, "Contract call reverted"; "reason" => reason);
            Ok(None)
        }

        // Any error reported by the Ethereum node could be due to the block no longer being on
        // the main chain. This is very unespecific but we don't want to risk failing a
        // subgraph due to a transient error such as a reorg.
        Err(EthereumContractCallError::Web3Error(e)) => Err(HostExportError::PossibleReorg(anyhow::anyhow!(
                "Ethereum node returned an error when calling function \"{}\" of contract \"{}\": {}",
                unresolved_call.function_name,
                unresolved_call.contract_name,
                e
            ))),

        // Also retry on timeouts.
        Err(EthereumContractCallError::Timeout) => Err(HostExportError::PossibleReorg(anyhow::anyhow!(
                "Ethereum node did not respond when calling function \"{}\" of contract \"{}\"",
                unresolved_call.function_name,
                unresolved_call.contract_name,
            ))),

        Err(e) => Err(HostExportError::Unknown(anyhow::anyhow!(
                "Failed to call function \"{}\" of contract \"{}\": {}",
                unresolved_call.function_name,
                unresolved_call.contract_name,
                e
            ))),
    };

    trace!(logger, "Contract call finished";
              "address" => &unresolved_call.contract_address.to_string(),
              "contract" => &unresolved_call.contract_name,
              "function" => &unresolved_call.function_name,
              "function_signature" => &unresolved_call.function_signature,
              "time" => format!("{}ms", start_time.elapsed().as_millis()));

    result
}
*/
