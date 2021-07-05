# Massbit Indexer

## Create new index with IPFS
- Start IPFS + Postgres docker
  ```shell
    cd indexer 
    docker-compose up
  ```
- Create the SO file by running 
  ```shell
    cargo install diesel_cli
    export DATABASE_URL="postgres://graph-node:let-me-in@localhost"
    diesel migration run
    cargo build --release
  ```
- Upload /target/release/libblock.so (built by first step) to http://0.0.0.0:5001/webui
- Start Substrate Node from https://github.com/scs/substrate-api-client-test-node
- Start Chain Reader 
  ```shell
    cd chain-reader
    cargo run --bin chain-reader 
  ```
  
- Start Index Manager
  ```
     cargo run --bin index-manager-main
  ```
- Create a new index deployment, replace hash_mapping_file with the hash from the step above
  ```http request
     curl --location --request POST 'localhost:3030' --header 'Content-Type: application/json' --data-raw '{"jsonrpc": "2.0", "method": "index_deploy_ipfs", "params": ["index_name","hash_project_yaml", "hash_mapping_file"], "id":1 }'
  ```
- After that, there will be index data in the Postgres database.

## Create new index with files from local 
To be added

