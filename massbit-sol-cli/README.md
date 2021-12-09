## RUN chain-reader and index-manager indexer
```bash
tmux new -d -s services "make services-dev-up"
tmux new -d -s chain-reader scripts/run-chain-reader.sh
tmux new -d -s indexer-api scripts/run-indexer-api.sh
```

## Install `massbit-sol`
```bash
cargo install massbit-sol
```

## Run CLI for gencode
```bash
cargo run --bin massbit-sol  -- gencode -s user-example/solana/instructions/serum/instruction.json -o code-compiler/generated/serum-index -c user-example/solana/instructions/serum/config.json
```
or 
```bash
massbit-sol gencode -s user-example/solana/instructions/serum/instruction.json -o code-compiler/generated/serum-index -c user-example/solana/instructions/serum/config.json
```
## Build indexer
```bash
cd serum-index
cargo build --release
```

## Deploy indexer
```bash
cd ../../../
cargo run --bin massbit-sol  -- deploy -u http://127.0.0.1:3031/indexers/deploy -d ~/Massbit/massbitprotocol/code-compiler/generated/serum-index
```
or
```bash
cd ../../../
massbit-sol deploy -u http://127.0.0.1:3031/indexers/deploy -d ~/Massbit/massbitprotocol/code-compiler/generated/serum-index
```

## Create releases folder for deploy from FE
```bash
cargo run --bin massbit-sol  -- release -d ~/Massbit/massbitprotocol/code-compiler/generated/serum-index
```
or
```bash
massbit-sol release -d ~/Massbit/massbitprotocol/code-compiler/generated/serum-index
```

## Publish new version of `massbit-sol` to `crate.io` 
(for Massbit developer only)
```bash
cd massbit-sol-cli
cargo publish
```