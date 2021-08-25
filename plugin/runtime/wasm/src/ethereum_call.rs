use graph::blockchain::{HostFn, HostFnCtx};

use crate::prelude::{Arc, Version};
use graph::runtime::{asc_get, asc_new, HostExportError};
use graph_chain_ethereum::runtime::abi::{
    AscUnresolvedContractCall, AscUnresolvedContractCall_0_0_4,
};
use graph_chain_ethereum::runtime::runtime_adapter::UnresolvedContractCall;
use graph_chain_ethereum::DataSource;
use graph_runtime_wasm::asc_abi::class::{AscEnumArray, EthereumValueKind};
use massbit_common::prelude::ethabi::{Token, Uint};

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
