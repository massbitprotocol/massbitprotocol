//! RPC interface for the transaction payment module.

use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
use sum_storage_runtime_api::SumStorageApi as SumStorageRuntimeApi;
use frame_support::Parameter;
#[rpc]
pub trait SumStorageApi<BlockHash,AccountId> {
	#[rpc(name = "massbit_getWorkers")]
	fn get_workers(&self, at: Option<BlockHash>) -> Result<Vec<(u32,Vec<u8>, AccountId,u32)>>;
	#[rpc(name = "massbit_getJobReports")]
	fn get_job_reports(&self, at: Option<BlockHash>) -> Result<Vec<(u32,u32,u32)>>;
	
}

/// A struct that implements the `SumStorageApi`.
pub struct SumStorage<C, M> {
	// If you have more generics, no need to SumStorage<C, M, N, P, ...>
	// just use a tuple like SumStorage<C, (M, N, P, ...)>
	client: Arc<C>,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> SumStorage<C, M> {
	/// Create new `SumStorage` instance with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

/// Error type of this RPC api.
// pub enum Error {
// 	/// The transaction was not decodable.
// 	DecodeError,
// 	/// The call to runtime failed.
// 	RuntimeError,
// }
//
// impl From<Error> for i64 {
// 	fn from(e: Error) -> i64 {
// 		match e {
// 			Error::RuntimeError => 1,
// 			Error::DecodeError => 2,
// 		}
// 	}
// }

impl<C, Block, AccountId> SumStorageApi<<Block as BlockT>::Hash, AccountId> for SumStorage<C, Block>
where
	Block: BlockT,
	AccountId: Parameter,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: SumStorageRuntimeApi<Block, AccountId>,
{
	fn get_workers(&self, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<(u32,Vec<u8>, AccountId,u32)>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));

		let runtime_api_result = api.get_workers(&at);
		runtime_api_result.map_err(|e| RpcError {
			code: ErrorCode::ServerError(1000), // No real reason for this value
			message: "Something wrong".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
	fn get_job_reports(&self, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<(u32,u32,u32)>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));

		let runtime_api_result = api.get_job_reports(&at);
		runtime_api_result.map_err(|e| RpcError {
			code: ErrorCode::ServerError(2000), // No real reason for this value
			message: "Something wrong".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

}
