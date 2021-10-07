# E2E Test for Substrate and Solana template

## Long tests
### Prerequisites
- start docker-compose.min.yml
  ```shell
  docker-compose -f docker-compose.min.yml up -d
  ```
- start chain-reader
   ```shell
  tmux new -s chain-reader 
  RUST_LOG_TYPE=file cargo run --bin chain-reader
  ```

### analytics tests:
- start analytics server
  ```shell
  tmux new -s ethereum-analytics 
  RUST_LOG=info cargo run --bin analytics -- -c ethereum -n matic -b 15000000
  tmux new -s solana-analytics 
  RUST_LOG=info cargo run --bin analytics -- -c solana -n mainnet -b 85000000
  ```

### indexer tests
- Harmony
  - start code-compiler
  ```shell
  cd code-compiler
  tmux new -s code-compiler
  python3 app.py
   ```
  - start indexer-v2
  ```
  tmux new -s indexer-v2
  cargo run --package manager --bin manager  -- --postgres-url postgresql://graph-node:let-me-in@localhost:5432/graph-node --ethereum-rpc mainnet:no_eip1898,archive,traces:https://a.api.s0.t.hmny.io --ipfs 127.0.0.1:5001 
  ```
  - run harmony e2e test
  ```
  cd e2e-test/ethereum
  robot -t "Compile and Deploy WASM Test Harmony" contract.robot
  ```


## Testing coverage
Core tests:
- BSC has 1 contract test (quickswap)
- Ethereum has 3 basic tests (block, transaction, event)
- Polygon has 1 contract test (quickswap), 1 chain test (transaction metrics)
- Solana has 3 basic tests (block, transaction, log messages), 2 advanced tests (serum, five tables to test compound type)
- Substrate has 3 basic tests (block, transaction, event)
- Cardano has none tests yet.

Health check tests:
- code-compiler 
- index-manager 
- dashboard 
- hasura graphql-engine 
- hasura console
- solana proxy 
- IPFS 
- chain-reader (to be added)
- Postgres DB (to be added)

Frontend Selenium tests:
- Dashboard

Production tests:
- Can be used when metric / logging is not enabled in the production server

Detail testing plan: https://app.gitbook.com/@hughie/s/massbit/e2e-test-planning

## Prerequisites
```shell
cd [to_project_root]
make test-init
make create-git-hook  # optional
```
- Make sure you have started all the services 
- If you don't want tests to run in every git push, you can run `make remove-all-git-hook`


## Run test
E2E tests have 4 categories:
- Basic: test block, event, transaction
- Advanced: compound type,...
- Chain: get metrics from the chain (trading volume, active address from BSC/Ethereum/Polygon)
- Contract: get metrics from the contract (pancakeswap, uniswap)

```shell
robot [test-type].robot
```

Example
```
robot basic.robot 
robot advanced.robot 
robot chain.robot 
robot contract.robot 
```

## Note
Because of our current test design, when we run the tests it will automatically delete the tables in the DB.
Doing so will affect the Index Manager when the INDEX_MANAGER_RESTART_INDEX option is enabled because the Index Manager will look for those tables when it restarts.

So make sure INDEX_MANAGER_RESTART_INDEX is set to false, or we need to redesign our tests, so they don't delete migrations table and index detail tables

## Log
Open log.html in your browser