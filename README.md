# Massbit Indexer
The goal of Massbit Indexer is to bring scalability and interoperability to Indexers.
In order to embrace the existing popular community and advanced technology, it will bring huge benefits by staying compatible with all the existing indexing mapping logics.
And to achieve that, the easiest solution is to develop with some existing features from the-graph, as we respect the great work of the-graph very much.

## Prerequisites
- Docker
- Python

```shell
make init-docker
make init-python
make init-test
```

## Hardware requirements
Running with public BSC/Ethereum/Solana node
- CPU: 16 cores
- Ram: 32 GB
- SSD or HDD

Running with local BSC/Ethereum/Solana node
- Use the hardware recommendation from https://docs.solana.com/running-validator/validator-reqs

## How to start
Running with public BSC/Ethereum/Solana Node
```shell
make services-prod-up
make index-quickswap   # To start indexing Quickswap on Polygon Chain
```

Running with local BSC/Ethereum/Solana Node
- Start your BSC/Ethereum/Solana node
- Modify chain-reader/chain-reader/src/lib.rs pointing to your local ws and http url
- ```shell
  make services-prod-up
  make index-quickswap   # To start indexing Quickswap on Polygon Chain
  ```

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