use anyhow::{Context, Error};
use blockchain::HostFn;
use ethabi::{Address, Token};
use massbit::runtime::{AscIndexId, IndexForAscTypeId};
use massbit::{
    blockchain::{self, BlockPtr, HostFnCtx},
    cheap_clone::CheapClone,
    prelude::Future01CompatExt,
    runtime::{asc_get, asc_new, AscPtr, HostExportError},
    semver::Version,
};
use runtime_wasm::asc_abi::class::{AscEnumArray, EthereumValueKind};
use std::{sync::Arc, time::Instant};

use super::abi::{AscUnresolvedContractCall, AscUnresolvedContractCall_0_0_4};
use crate::data_source::MappingABI;
use crate::network::EthereumNetworkAdapters;
use crate::{
    Chain, DataSource, EthereumAdapter, EthereumAdapterTrait, EthereumContractCall,
    EthereumContractCallError,
};

pub struct RuntimeAdapter {
    pub(crate) eth_adapters: Arc<EthereumNetworkAdapters>,
}

impl blockchain::RuntimeAdapter<Chain> for RuntimeAdapter {
    fn host_fns(&self, ds: &DataSource) -> Result<Vec<HostFn>, Error> {
        let abis = ds.mapping.abis.clone();
        let eth_adapter = self
            .eth_adapters
            .cheapest()
            .with_context(|| "no adapter for chain")?
            .cheap_clone();

        let ethereum_call = HostFn {
            name: "ethereum.call",
            func: Arc::new(move |ctx, wasm_ptr| {
                ethereum_call(&eth_adapter, ctx, wasm_ptr, &abis).map(|ptr| ptr.wasm_ptr())
            }),
        };

        Ok(vec![ethereum_call])
    }
}

/// function ethereum.call(call: SmartContractCall): Array<Token> | null
fn ethereum_call(
    eth_adapter: &EthereumAdapter,
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

    let result = eth_call(eth_adapter, &ctx.block_ptr, call, abis)?;
    match result {
        Some(tokens) => Ok(asc_new(ctx.heap, tokens.as_slice())?),
        None => Ok(AscPtr::null()),
    }
}

/// Returns `Ok(None)` if the call was reverted.
fn eth_call(
    eth_adapter: &EthereumAdapter,
    block_ptr: &BlockPtr,
    unresolved_call: UnresolvedContractCall,
    abis: &[Arc<MappingABI>],
) -> Result<Option<Vec<Token>>, HostExportError> {
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
        address: unresolved_call.contract_address,
        block_ptr: block_ptr.cheap_clone(),
        function: function.clone(),
        args: unresolved_call.function_args.clone(),
    };

    // Run Ethereum call in tokio runtime
    let result = match massbit::block_on(
        eth_adapter.contract_call(call).compat()
    ) {
        Ok(tokens) => Ok(Some(tokens)),
        Err(EthereumContractCallError::Revert(reason)) => {
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

    result
}

#[derive(Clone, Debug)]
pub struct UnresolvedContractCall {
    pub contract_name: String,
    pub contract_address: Address,
    pub function_name: String,
    pub function_signature: Option<String>,
    pub function_args: Vec<ethabi::Token>,
}

impl AscIndexId for AscUnresolvedContractCall {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::SmartContractCall;
}
