#![cfg_attr(not(feature = "std"), no_std)]


use codec::{Encode, Decode};
use frame_support::{
	decl_module, decl_storage, decl_event, decl_error, ensure, StorageValue, StorageDoubleMap, Parameter,
/*traits::Randomness,*/ RuntimeDebug, dispatch::{DispatchError, DispatchResult},
};

use frame_system::ensure_signed;
use sp_runtime::traits::{AtLeast32BitUnsigned, Bounded,  One, CheckedAdd};
use sp_std::vec::Vec;


const AGREE_THREADHOLD_RATIO:f32 = 0.5;
//const DENY_THREADHOLD_RATIO:f32 = 0.3;

#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq)]
pub enum WorkerStatus {
	NormalStatus,
	BlackList
}

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Worker<JobProposalIndex>{
	pub ip: Vec<u8>,
	pub status: WorkerStatus,
	pub job_proposal_id: JobProposalIndex,
}
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct JobProposal<AccountId>{
	pub proposer_account_id: AccountId,
	pub name: Vec<u8>, 
	pub stake: u64, 
	pub description: Vec<u8>, 
	pub call_url: Vec<u8>, 
}

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct JobReport<WorkerIndex,AccountId>{
	pub responsible_account_id: AccountId,
	pub responsible_worker_id: WorkerIndex,
	pub job_input: Vec<u8>,
	pub job_output: Vec<u8>,
	pub verify_agree_workers: Vec<WorkerIndex>,
	pub verify_deny_workers: Vec<WorkerIndex>,
	pub client_account: AccountId,
}

pub trait Trait: pallet_balances::Trait{
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type WorkerIndex: Parameter + AtLeast32BitUnsigned + Bounded + Default + Copy;
	type JobReportIndex: Parameter + AtLeast32BitUnsigned + Bounded + Default + Copy;
	type JobProposalIndex: Parameter + AtLeast32BitUnsigned + Bounded + Default + Copy;
	type JobRequestUrl: Parameter;
	type JobResponse: Parameter;
}
decl_storage! {
	trait Store for Module<T: Trait> as MassbitModule {
		/// Stores all the workers
		pub Workers get(fn workers): double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) T::WorkerIndex => Option<Worker<T::JobProposalIndex>>;
		/// Stores the workers number
		pub ActiveWorkerCount: u32;
		/// Stores the next worker ID
		pub NextWorkerId get(fn next_worker_id): T::WorkerIndex;
		/// Stores job reports
		pub JobReports get(fn job_reports): map hasher(blake2_128_concat) T::JobReportIndex => Option<JobReport<T::WorkerIndex,T::AccountId>>;
		/// Stores the next job ID
		pub NextJobReportId get(fn next_job_report_id): T::JobReportIndex;
		/// Store Proposal 
		pub JobProposals get(fn job_proposals): map hasher(blake2_128_concat) T::JobProposalIndex => Option<JobProposal<T::AccountId>>;
		/// Stores the next Proposal ID
		pub NextJobProposalId get(fn next_job_proposal_id): T::JobProposalIndex;
	}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		<T as Trait>::WorkerIndex,
		<T as Trait>::JobReportIndex,
		<T as Trait>::JobProposalIndex,
		//<T as pallet_balances::Trait>::Balance,
	{
		/// A Worker is registered. \[owner, worker_id, worker, active_worker_count\]
		WorkerRegistered(AccountId, WorkerIndex, Worker<JobProposalIndex>, u32),
		/// A report Worker is saved. \[owner, job_report_id, job_report\]
		JobReportSaved(AccountId, JobReportIndex, JobReport<WorkerIndex,AccountId>),
		/// A vote on report is saved. \[owner, job_report_id, resposible_worker, activate_worker_count\]
		JobReportVoteSaved(AccountId, JobReportIndex, Worker<JobProposalIndex>, u32),
		/// A vote on report is saved. \[job_report_id, job_report, responsive_worker,responsive_account, activate_worker_count\]
		JobReportProcessFinished(JobReportIndex, JobReport<WorkerIndex,AccountId>, WorkerIndex, AccountId, u32),
		/// A job proposal is create. \[job_prposal_id, job_prposal_id\]
		JobProposalRegistered(JobProposalIndex, JobProposal<AccountId>),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		WorkersIdOverflow,
		JobReportIdOverflow,
		NotRegisteredWorker,
		InvalidJobReportId,
		AlreadyVoteWorker,
		JobProposalIdOverflow,
		NotRegisteredJobProposal,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Create a new worker
		#[weight = 1000]
		pub fn register_worker(origin, ip: Vec<u8>,job_proposal_id: T::JobProposalIndex) {
			let sender = ensure_signed(origin)?;
			ensure!(JobProposals::<T>::contains_key(&job_proposal_id),Error::<T>::NotRegisteredJobProposal);

			// TODO(huy): call http to check if the worker is live

			let worker_id = Self::get_next_worker_id()?;

			// Create and store worker
			let worker = Worker{
				ip: ip,
				status: WorkerStatus::NormalStatus,
				job_proposal_id: job_proposal_id,
			};
			Workers::<T>::insert(&sender, worker_id, worker.clone());
			
			// Increase WorkerCount
			ActiveWorkerCount::mutate(|v| *v += 1);
			let active_worker_count = ActiveWorkerCount::get();

			// Emit event
			Self::deposit_event(RawEvent::WorkerRegistered(sender, worker_id, worker, active_worker_count))
		}

		/// Create a new report
		#[weight = 1000]
		pub fn submit_report(origin, responsible_account_id: T::AccountId, responsible_worker_id: T::WorkerIndex, job_input: Vec<u8>, job_output: Vec<u8>) {
			let sender = ensure_signed(origin)?;

			// Checking if responsible_worker_id is created from responsible_account_id
			ensure!(Workers::<T>::contains_key(&responsible_account_id,&responsible_worker_id),Error::<T>::NotRegisteredWorker);

			let job_report_id = Self::get_next_job_report_id()?;

			// TODO(huy): add signature of the job_output 

			let job_report = JobReport{
				responsible_account_id : responsible_account_id,
				responsible_worker_id : responsible_worker_id,
				job_input : job_input,
				job_output : job_output,
				verify_deny_workers: Vec::new(),
				verify_agree_workers: Vec::new(),
				client_account: sender.clone(),
			};

			JobReports::<T>::insert(job_report_id, job_report.clone());
			// Emit event
			Self::deposit_event(RawEvent::JobReportSaved(sender, job_report_id, job_report))
		}

		/// Add a vote report
		#[weight = 1000]
		pub fn vote_job_report(origin, voted_worker_id: T::WorkerIndex, job_report_id: T::JobReportIndex, verify_agree: bool) {
			let sender = ensure_signed(origin)?;
			// Check worker id
			ensure!(Workers::<T>::contains_key(&sender,voted_worker_id),Error::<T>::NotRegisteredWorker);
			// Check report id
			ensure!(JobReports::<T>::contains_key(&job_report_id),Error::<T>::InvalidJobReportId);
			// Check already vote
			let mut clone_job_report = JobReports::<T>::get(&job_report_id).unwrap().clone();
			ensure!(!clone_job_report.verify_agree_workers.contains(&voted_worker_id) && !clone_job_report.verify_deny_workers.contains(&voted_worker_id),Error::<T>::AlreadyVoteWorker);
			
			JobReports::<T>::try_mutate_exists(&job_report_id, |job_report| -> DispatchResult{
				
				//let mut job_report = job_report.unwrap();
				let total_workers = ActiveWorkerCount::get();
				let mut is_delete_report = false;
				// Update vote vec
				if verify_agree{
					clone_job_report.verify_agree_workers.push(voted_worker_id);
					let agree_number = clone_job_report.verify_agree_workers.len();
					if (agree_number as f32) > AGREE_THREADHOLD_RATIO*(total_workers as f32) {
						// Remove the job report
						let _job_report = job_report.take().ok_or(Error::<T>::InvalidJobReportId)?;
						is_delete_report = true;
					}
				}
				else{
					clone_job_report.verify_deny_workers.push(voted_worker_id);
					let deny_number = clone_job_report.verify_deny_workers.len();
					if (deny_number as f32) >= (1f32-AGREE_THREADHOLD_RATIO)*(total_workers as f32) {
						// Remove the job report
						let _job_report = job_report.take().ok_or(Error::<T>::InvalidJobReportId)?;
						is_delete_report = true;
						// Add response worker to blacklist
						Self::add_worker_to_blacklist(&clone_job_report.responsible_account_id,&clone_job_report.responsible_worker_id)?;
					}
				}

				let reponsive_worker = Workers::<T>::get(&clone_job_report.responsible_account_id,&clone_job_report.responsible_worker_id).unwrap();
				if !is_delete_report {
					*job_report = Some(clone_job_report);
				}
				
				// Emit event
				Self::deposit_event(RawEvent::JobReportVoteSaved(sender, job_report_id, reponsive_worker , ActiveWorkerCount::get()));

				Ok(())
			})?;
		}
		
		/// Register a new job proposal
		#[weight = 1000]
		pub fn regiter_job_proposal(origin, name: Vec<u8>, stake: u64, description: Vec<u8>, call_url: Vec<u8>) {
			let sender = ensure_signed(origin)?;

			let job_proposal_id = Self::get_next_job_proposal_id()?;

			let job_proposal = JobProposal{
				name: name, 
				stake: stake, 
				description: description, 
				call_url: call_url, 
				proposer_account_id: sender,
			};

			JobProposals::<T>::insert(job_proposal_id, job_proposal.clone());
			// Emit event
			Self::deposit_event(RawEvent::JobProposalRegistered(job_proposal_id, job_proposal))
		}		
	}
}


impl<T: Trait> Module<T> {
	fn get_next_worker_id() -> sp_std::result::Result<T::WorkerIndex, DispatchError> {
		NextWorkerId::<T>::try_mutate(|next_id| -> sp_std::result::Result<T::WorkerIndex, DispatchError> {
			let current_id = *next_id;
			*next_id = next_id.checked_add(&One::one()).ok_or(Error::<T>::WorkersIdOverflow)?;
			Ok(current_id)
		})
	}
	fn get_next_job_report_id() -> sp_std::result::Result<T::JobReportIndex, DispatchError> {
		NextJobReportId::<T>::try_mutate(|next_id| -> sp_std::result::Result<T::JobReportIndex, DispatchError> {
			let current_id = *next_id;
			*next_id = next_id.checked_add(&One::one()).ok_or(Error::<T>::JobReportIdOverflow)?;
			Ok(current_id)
		})
	}

	
	fn add_worker_to_blacklist(responsible_account_id: &T::AccountId,responsible_worker_id: &T::WorkerIndex)-> DispatchResult{
		// Clone the worker
		let mut clone_worker = Workers::<T>::get(&responsible_account_id, &responsible_worker_id).clone().unwrap();
		Workers::<T>::try_mutate_exists(&responsible_account_id, &responsible_worker_id, |worker| -> DispatchResult{
			// Set worker status to BlackList
			clone_worker.status = WorkerStatus::BlackList;
			*worker = Some(clone_worker);
			Ok(())
		})?;

		// Reduce number of activate worker
		ActiveWorkerCount::mutate(|v| *v -= 1);
		Ok(())
	} 

	fn get_next_job_proposal_id() -> sp_std::result::Result<T::JobProposalIndex, DispatchError> {
		NextJobProposalId::<T>::try_mutate(|next_id| -> sp_std::result::Result<T::JobProposalIndex, DispatchError> {
			let current_id = *next_id;
			*next_id = next_id.checked_add(&One::one()).ok_or(Error::<T>::JobProposalIdOverflow)?;
			Ok(current_id)
		})
	}
}

impl<T: Trait> Module<T> {
	pub fn get_workers() -> Vec<(T::WorkerIndex,Vec<u8>,T::AccountId, bool, T::JobProposalIndex)> {
		let mut vec_workers = Vec::new();
		
		for  (_account_id, worker_id, v) in Workers::<T>::iter() {
			vec_workers.push((worker_id, v.ip, _account_id, v.status==WorkerStatus::NormalStatus, v.job_proposal_id));	
		}
		vec_workers
	}
	pub fn get_job_reports() -> Vec<(T::JobReportIndex,Vec<u8>,Vec<u8>)> {
		let mut vec_job_reports = Vec::new();
		
		for  (k, v) in JobReports::<T>::iter() {
			vec_job_reports.push((k,v.job_input,v.job_output));	
		}
		vec_job_reports
	}

	pub fn get_job_proposals() -> Vec<(T::JobProposalIndex, T::AccountId, Vec<u8>, u64, Vec<u8>, Vec<u8>)> {
		let mut vec_job_proposals = Vec::new();
		
		for  (job_proposal_index, v) in JobProposals::<T>::iter() {
			vec_job_proposals.push((job_proposal_index, v.proposer_account_id, v.name, v.stake, v.description, v.call_url));
		}
		vec_job_proposals
	}
}
