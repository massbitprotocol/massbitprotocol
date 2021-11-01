## Prerequisites
The store (s3 / ipfs / local) should contain 
- a config file (project.yaml)

```yaml
schema:
  file: ./schema.graphql

dataSources:
  - kind: solana
    name: Index
    network: https://data-seed-prebsc-1-s1.binance.org:8545/
    mapping:
      language: rust
      handlers:
        - handler: handleBlock
          kind: solana/BlockHandler
```

- a schema (schema.graphql / rust model)
```
type IndexSchema @entity{
  id: ID!
  account: BigInt!
  date: Date!
}
```

- a mapping file (mapping.rs)
```
use SolanaBlock;
use std::time::{SystemTime, UNIX_EPOCH};

fn handleBlock(block: SolanaBlock) {
    // Add user logic here
    block.date = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    block.save();
}

```

## API

Method: index_deploy (with type = local)

Updated on: 6-7-2021

Description: 
- For every new request, the indexer will start a new thread to index data.
- params: 
  - Name of the index
  - The path to the local project.yaml
  - The path to the local .SO mapping file
  - The path to the index model / SQL schema
  - The type of deployment (Local)

```http request
curl --location --request POST 'localhost:3030' --header 'Content-Type: application/json' --data-raw '{"jsonrpc": "2.0", "method": "index_deploy_local", "params": ["index_name","../example/project.yaml", "../example/libblock.so", "../example/up.sql", "Local"], "id":1 }'
```

Method: index_deploy (with type = IPFS)

Updated on: 6-7-2021 

Description:
- For every new request, the indexer will start a new thread to index data.
- params:
  - Name of the index
  - The IPFS Hash of the project.yaml
  - The IPFS Hash of the .SO mapping file
  - The path to the index model / SQL schema
  - The type of deployment (Ipfs)

```http request
curl --location --request POST 'localhost:3030' --header 'Content-Type: application/json' --data-raw '{"jsonrpc": "2.0", "method": "index_deploy", "params": ["index_name","hash_project_yaml", "hash_mapping_file", "hash_model_file", "Ipfs"], "id":1 }'
```