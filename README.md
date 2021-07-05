# Massbit Indexer
The goal of Massbit Indexer is to bring scalability and interoperability to Indexers. 
In order to embrace the existing popular community and advanced technology, it will bring huge benefits by staying compatible with all the existing indexing mapping logics. 
And to achieve that, the easiest solution is to develop with some existing features from the-graph, as we respect the great work of the-graph very much.

## Create new index with IPFS
- Start IPFS + Postgres docker
  ```shell
    cd indexer 
    docker-compose up
  ```
- Create the SO file by running 
  ```shell
    cargo build --release
  ```
  
- Run migration manually to create the database (this will be handled automatically by index-manager later)
  ```shell
    cd plugin-examples/block
    cargo install diesel_cli
    export DATABASE_URL="postgres://graph-node:let-me-in@localhost"
    diesel migration run
  ```
- Upload 
    - /target/release/libblock.so (built by the previous step) to http://0.0.0.0:5001/webui
    - indexer/example/project.yaml to http://0.0.0.0:5001/webui
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
- After that, there will be the indexed data in the Postgres database.

## Create new index with files from local 
To be added

