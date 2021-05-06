#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_mut_passed)]
use sp_std::vec::Vec;

// Here we declare the runtime API. It is implemented it the `impl` block in
// runtime amalgamator file (the `runtime/src/lib.rs`)
sp_api::decl_runtime_apis! {
	pub trait SumStorageApi {
		fn get_workers() -> Vec<Vec<u8>>;
		fn get_job_reports() -> Vec<(u32,u32,u32)>;
	}
}
