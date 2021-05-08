# Error
```
  --> node/src/rpc.rs:65:10
   |
65 |         crate::silly_rpc::SillyRpc::to_delegate(crate::silly_rpc::Silly {})
   |                ^^^^^^^^^ maybe a missing crate `silly_rpc`?
```
Fix by adding:
```
pub mod silly_rpc;
```
to file `node/src/lib.rs`.

# RPC call

```
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"massbit_getJobReports",
      "params": []
    }'
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"massbit_getWorkers",
      "params": []
    }'
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"massbit_getJobProposals",
      "params": []
    }'
```
# Demo
## Clone project
```
git clone https://github.com/codelight-co/substrate-learning.git
```
## Run Massbit
```
cd substrate-learning/massbit
make run
```
## Config custom type data
https://polkadot.js.org/apps/#/settings/developer
Add the following code into `Additional types as a JSON file (or edit below)`:
```
{
  "WorkerStatus": {
    "_enum": [
      "NormalStatus",
      "BlackList"
    ]
  },
  "Worker": {
    "ip": "Vec<u8>",
    "status": "WorkerStatus",
    "job_proposal_id": "JobProposalIndex"
  },
  "WorkerIndex": "u32",
  "JobReportIndex": "u32",
  "JobProposalIndex": "u32",
  "JobReport": {
    "responsible_account_id": "AccountId",
    "responsible_worker_id": "WorkerIndex",
    "job_input": "Vec<u8>",
    "job_output": "Vec<u8>",
    "verify_agree_workers": "Vec<WorkerIndex>",
    "verify_deny_workers": "Vec<WorkerIndex>",
    "client_account": "AccountId"
  },
  "JobProposal": {
    "proposer_account_id": "AccountId",
    "name": "Vec<u8>",
    "stake": "u64",
    "description": "Vec<u8>",
    "call_url": "Vec<u8>"
  }
}
```
## Create 5 worker
### Action
Go to https://polkadot.js.org/apps/#/extrinsics
-> Select account `ALICE`
-> in `submit the following extrinsic` select `massbit` -> select `create(ip)` -> input into `ip` field the value `0001`
-> click `Submit Transaction`
-> click `Sign and Submit`

input into `ip` field the value `0002`
-> click `Submit Transaction`
-> click `Sign and Submit`

Repeat until create 5 workers

### Result
-> massbit.activeWorkerCount = 5

## Submit a job report 
### Action
-> in `submit the following extrinsic` select `massbit` -> select `saveJobReport` -> input `responsible_worker_id`: `0` 
-> input `job_input`: `1` -> input `job_output`: `1`
-> click `Submit Transaction`
-> click `Sign and Submit`

### Result
-> Job report created


## Vote a job report 
### Action
-> in `submit the following extrinsic` select `massbit` -> select `voteJobReport` -> input `voted_worker_id`: `1` 
-> input `job_report_id`: `0` -> input `verify_agree`: `No`
-> click `Submit Transaction`
-> click `Sign and Submit`

-> in `submit the following extrinsic` select `massbit` -> select `voteJobReport` -> input `voted_worker_id`: `2` 
-> input `job_report_id`: `0` -> input `verify_agree`: `No`
-> click `Submit Transaction`
-> click `Sign and Submit`

-> in `submit the following extrinsic` select `massbit` -> select `voteJobReport` -> input `voted_worker_id`: `3` 
-> input `job_report_id`: `0` -> input `verify_agree`: `No`
-> click `Submit Transaction`
-> click `Sign and Submit`

### Result
-> Job report delete
-> Worker id 0: blacklist
-> massbit.activeWorkerCount = 4

