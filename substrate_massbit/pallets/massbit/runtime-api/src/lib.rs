#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_mut_passed)]
use sp_std::vec::Vec;
use frame_support::Parameter;
// Here we declare the runtime API. It is implemented it the `impl` block in
// runtime amalgamator file (the `runtime/src/lib.rs`)
sp_api::decl_runtime_apis! {
	pub trait MassbitApi<AccountId,WorkerIndex,JobProposalIndex> where
	AccountId: Parameter,
	WorkerIndex: Parameter,
	JobProposalIndex: Parameter,
	{
		fn get_workers() -> Vec<(WorkerIndex,Vec<u8>,AccountId,JobProposalIndex)>;
		fn get_job_reports() -> Vec<(u32,u32,u32)>;
	}
}
