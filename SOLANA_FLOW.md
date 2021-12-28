# Solana Frame work
## Developer 
1. Create `instruction.json` and `config.json`, or use examples at https://github.com/massbitprotocol/massbitprotocol/tree/main/user-example/solana/instructions .
2. Install massbit-sol
```
cargo install massbit-sol
```
3. Run code-gen
```
massbit-sol codegen -s instruction.json -c serum_config.json -o ./indexer 
cd indexer
```
4. (Optional) Edit code in `indexer` folder 
5. Build `indexer` code
```
cargo build --release
```
7. Run service Chain-reader
By Github repo
```
git clone https://github.com/massbitprotocol/massbitprotocol.git
cd massbitprotocol
make run-code-compiler
make run-chain-reader
make run-indexer-manager
cd ..
```
Or docker image (update later)
```
```
8. Deploy 
```
massbit-sol deploy --url http://127.0.0.1:3032/indexer/deploy --directory indexer
```
9. Check indexed data at
```
http://127.0.0.1:3002/
```

## Indexer
1. Create/Reuse github repo with structure such as:
```
.
└── release
     └── v0.1.0
          ├── schema.graphql
          ├── subgraph.yaml
          └── libblock.so
```
2. Input custom Indexer information, repo URL at Massbit Indexer website (update url later)
3. Click `Deploy`
4. Check data at Massbit Indexer website (update url later)
