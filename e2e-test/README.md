# E2E Test for Substrate and Solana template

## Testing plans
Solana
- block (done)
- transaction (done)
- log message (done)

Substrate
- block (done)
- extrinsic (done)
- event (done)

Health check for all services
- code-compiler (done)
- chain-reader
- index-manager (done)
- dashboard
- hasura graphql-engine
- hasura console
- solana proxy
- IPFS
- Postgres DB

Detail testing plan: https://app.gitbook.com/@hughie/s/massbit/e2e-test-planning

## Prerequisites
```
pip install robotframework-requests
pip install robotframework-databaselibrary
```
And make sure you have started all the services 

## Run a test
```
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