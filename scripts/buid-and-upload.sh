#! /bin/sh
docker build --tag anhhuy0501/chain-reader:0.1 -f deployment/chain-reader/Dockerfile -t chain-reader .;
docker build --tag anhhuy0501/indexer-manager:0.1 -f deployment/indexer-manager/Dockerfile -t indexer-manager .;
docker build --tag anhhuy0501/indexer-api:0.1 -f deployment/indexer-api/Dockerfile -t indexer-api .;

docker push anhhuy0501/chain-reader:0.1;
docker push anhhuy0501/indexer-manager:0.1;
docker push anhhuy0501/indexer-api:0.1;
