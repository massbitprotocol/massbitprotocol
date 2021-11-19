## RUN chain-reader and index-manager indexer
```bash
tmux new -d -s services "make services-dev-up"
tmux new -d -s chain-reader scripts/tmux-chain-reader.sh
tmux new -d -s indexer-v1 scripts/tmux-indexer-v1.sh
tmux new -d -s indexer scripts/tmux-code-compiler.sh
```

## Run CLI
```bash
cargo run --bin massbit-sol  -- -s user-example/solana/instructions/serum/instruction.json -o serum-index -c user-example/solana/instructions/serum/config.json
```

## Build indexer
```bash
cd serum-index
cargo build --release
```

## Deploy indexer
```bash
cd ../../../
make deploy-so id=serum-index
```
