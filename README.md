# Massbit Indexer
The goal of Massbit Indexer is to bring scalability and interoperability to Indexers. 
In order to embrace the existing popular community and advanced technology, it will bring huge benefits by staying compatible with all the existing indexing mapping logics. 
And to achieve that, the easiest solution is to develop with some existing features from the-graph, as we respect the great work of the-graph very much.

## Prerequisites
- Docker

## Hardware requirements
Running with public BSC/Ethereum/Solana node
- CPU: 16 cores
- Ram: 32 GB
- SSD or HDD

Running with local BSC/Ethereum/Solana node
- https://docs.solana.com/running-validator/validator-reqs

## How to start 
Running with public BSC/Ethereum/Solana Node
```shell
docker-compose -f docker.compose.prod.yml up
make test-run-all
```

Running with local BSC/Ethereum/Solana Node
- Start your BSC/Ethereum/Solana node
- Modify chain-reader/chain-reader/src/lib.rs pointing to your local ws and http url
- ```
  docker-compose -f docker.compose.prod.yml up
  make test-run-all
  ```

## Dockerize (For Dev) 
### Auto create new docker version:
- Add the massbitprotocol repo, create a new release tag, it will automatically build new docker version and upload to docker hub 
- Check the new images here: https://registry.hub.docker.com/u/sprise

### Manually build other services:
- dashboard: Go to this repo https://github.com/massbitprotocol/dashboard and run the below commands
  - `docker build --tag sprise/dashboard:[new_version_id] -f Dockerfile -t dashboard .`
  - `docker push sprise/dashboard:[new_version_id]`
- massbit graphql-engine console: Go to this repo https://github.com/massbitprotocol/graphql-engine and run the below commands 
  - `docker build --tag sprise/hasura-console:[new_version_id] -f Dockerfile -t hasura-console .` 
  - `docker push sprise/hasura-console:[new_version_id]`

### Manually create new docker version with tag
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

## Development
### Run all services
Run all service with a single command. For testing purpose.
```bash
sh run.sh
```

### Deploy 
Deploy the indexer with id, in case the indexer's files already successfully build once. This is for reducing rebuild time.

```bash
make dev-deploy id=54e42a73317d80d1cf8289b49af96302
```
