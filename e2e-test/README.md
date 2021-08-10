# E2E Test for Substrate and Solana template

## Testing plans
Solana
- block (done)
- transaction (done)
- log message (done)
- index serum (done)
- five tables (done)

Substrate
- block (done)
- extrinsic (done)
- event (done)

Health check for all services
- code-compiler (done)
- index-manager (done)
- dashboard (done)
- hasura graphql-engine (done)
- hasura console (done)
- solana proxy (done)
- IPFS (done)
- chain-reader
- Postgres DB

Detail testing plan: https://app.gitbook.com/@hughie/s/massbit/e2e-test-planning

## Prerequisites
```
make test-init
```
And make sure you have started all the services 

## Run all test
```shell
make test-run-all
```

## Run a test
```shell
robot [test-name].robot
```
Example
```
robot substrate.robot 
robot solana.robot 
robot health-check.robot 
```

## Log
Open log.html in your browser