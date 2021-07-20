# Massbit Indexer
The goal of Massbit Indexer is to bring scalability and interoperability to Indexers. 
In order to embrace the existing popular community and advanced technology, it will bring huge benefits by staying compatible with all the existing indexing mapping logics. 
And to achieve that, the easiest solution is to develop with some existing features from the-graph, as we respect the great work of the-graph very much.

## To build and deploy docker-compose.prod.yml
Build new images
- Run `docker build --tag sprise/chain-reader:[new_version_id] -f chain-reader/Dockerfile -t chain-reader .` to build chain-reader
- Run `docker build --tag sprise/indexer:[new_version_id] -f indexer/Dockerfile -t indexer .` to build indexer
- Run `docker build --tag sprise/code-compiler:[new_version_id] -f code-compiler/Dockerfile -t code-compiler .` to build code-compiler
- Run `docker build --tag sprise/dashboard:[new_version_id] -f frontend/dashboard/Dockerfile -t dashboard .` to build the dashboard with the latest code from massbitprocol/dashboard git

Deploy new images to Docker Hub:
- `docker push sprise/chain-reader:[new_version_id]`
- `docker push sprise/indexer:[new_version_id]`
- `docker push sprise/code-compiler:[new_version_id]`
- `docker push sprise/dashboard:[new_version_id]`

Check the new images here: https://registry.hub.docker.com/u/sprise

To start those images in prod: `docker-compose -f docker-compose.prod.yml up`

Note:
- The 3 Rust services (substrate-node, chain-reader, indexer) we need to build them separately because their build time is long and we need wait-for-it.sh script implemented.
- The code-compiler needs to know the context of massbitprotcol source folder so it can run the `cargo build` for the /compile api

## Create new index with IPFS
- Start IPFS + Postgres docker
  ```shell
    cd indexer 
    docker-compose up
  
    docker exec -it replace_with_ipfs_image_name /bin/sh
    ipfs config --json API.HTTPHeaders.Access-Control-Allow-Origin '["http://0.0.0.0:5001", "http://127.0.0.1:5001", "https://webui.ipfs.io"]'
    ipfs config --json API.HTTPHeaders.Access-Control-Allow-Methods '["PUT", "GET", "POST"]'
    
    // Press ctrl + c to stop the docker, then start it again so IPFS will apply CORS config
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

## Update build-and-upload.sh script to build and deploy to DockerHub so we can save time on compiling 
- 
