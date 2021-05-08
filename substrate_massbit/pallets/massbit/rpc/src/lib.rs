//! RPC interface for the transaction payment module.

use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
use massbit_runtime_api::MassbitApi as MassbitRuntimeApi;
use frame_support::Parameter;

#[rpc]
pub trait MassbitApi<BlockHash,WorkerIndex,AccountId,JobProposalIndex> {
	#[rpc(name = "massbit_getWorkers")]
	fn get_workers(&self, at: Option<BlockHash>) -> Result<Vec<(WorkerIndex,Vec<u8>,AccountId, bool, JobProposalIndex)>>;
	#[rpc(name = "massbit_getJobReports")]
	fn get_job_reports(&self, at: Option<BlockHash>) -> Result<Vec<(u32,Vec<u8>,Vec<u8>)>>;
	#[rpc(name = "massbit_getJobProposals")]
	fn get_job_proposals(&self, at: Option<BlockHash>) -> Result<Vec<(JobProposalIndex, AccountId, Vec<u8>, u64, Vec<u8>, Vec<u8>)>>;
	
}


/// A struct that implements the `MassbitApi`.
pub struct Massbit<C, M> {
	// If you have more generics, no need to Massbit<C, M, N, P, ...>
	// just use a tuple like Massbit<C, (M, N, P, ...)>
	client: Arc<C>,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> Massbit<C, M> {
	/// Create new `Massbit` instance with the given reference to the client.
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

impl<C, Block, WorkerIndex, AccountId, JobProposalIndex> MassbitApi<<Block as BlockT>::Hash, WorkerIndex, AccountId, JobProposalIndex> for Massbit<C, Block>
where
	Block: BlockT,
	AccountId: Parameter,
	WorkerIndex: Parameter,
	JobProposalIndex: Parameter,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: MassbitRuntimeApi<Block, AccountId,WorkerIndex,JobProposalIndex>,
{
	fn get_workers(&self, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<(WorkerIndex,Vec<u8>,AccountId, bool, JobProposalIndex)>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));

		let runtime_api_result = api.get_workers(&at);
		runtime_api_result.map_err(|e| RpcError {
			code: ErrorCode::ServerError(1000), // No real reason for this value
			message: "Something wrong with get_workers".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
	fn get_job_reports(&self, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<(u32,Vec<u8>,Vec<u8>)>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));

		let runtime_api_result = api.get_job_reports(&at);
		runtime_api_result.map_err(|e| RpcError {
			code: ErrorCode::ServerError(2000), // No real reason for this value
			message: "Something wrong with get_job_reports".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
	fn get_job_proposals(&self, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<(JobProposalIndex, AccountId, Vec<u8>, u64, Vec<u8>, Vec<u8>)>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));

		let runtime_api_result = api.get_job_proposals(&at);
		runtime_api_result.map_err(|e| RpcError {
			code: ErrorCode::ServerError(3000), // No real reason for this value
			message: "Something wrong qith get_job_proposals".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

}
