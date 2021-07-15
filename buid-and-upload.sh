#! /bin/sh
docker build --tag sprise/substrate-node:0.1 -f substrate-node/Dockerfile -t substrate-node .;
#docker build --tag sprise/chain-reader:0.1 -f chain-reader/Dockerfile -t chain-reader .;
docker build --tag sprise/chain-reader:0.1 -f massbitprotocol/chain-reader/Dockerfile -t chain-reader .; # Use this from outside massbitprotocol folder when we need to import other Cargo project
#docker build --tag sprise/indexer:0.1 -f indexer/Dockerfile -t indexer .;
docker build --tag sprise/indexer:0.1 -f massbitprotocol/indexer/Dockerfile -t indexer .; # Use this from outside massbitprotocol folder when we need to import other Cargo project
docker build --tag sprise/code-compiler:0.1 -f code-compiler/Dockerfile -t code-compiler .;
docker build --tag sprise/dashboard:0.1 -f frontend/dashboard/Dockerfile -t dashboard .;

docker push sprise/substrate-node:0.1;
docker push sprise/chain-reader:0.1;
docker push sprise/indexer:0.1;
docker push sprise/code-compiler:0.1;
docker push sprise/dashboard:0.1;