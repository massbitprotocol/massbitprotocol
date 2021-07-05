# Massbit Indexer

## Start the full IPFS flow

- Create the SO file by running 
  ```
    cargo install diesel_cli
    export DATABASE_URL="postgres://graph-node:let-me-in@localhost"
    diesel migration run
    cargo build --release
  ```
- Start IPFS docker
  ```
    cd indexer 
    docker-compose up
  ```
- Upload /target/release/libblock.so (built by first step) to http://0.0.0.0:5001/webui
- Create a new index deployment, replace hash_mapping_file with the hash from the step above
  ```http request
  curl --location --request POST 'localhost:3030' --header 'Content-Type: application/json' --data-raw '{"jsonrpc": "2.0", "method": "index_deploy_ipfs", "params": ["index_name","hash_project_yaml", "hash_mapping_file"], "id":1 }'
  ```
