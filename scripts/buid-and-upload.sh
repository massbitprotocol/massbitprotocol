#! /bin/sh
docker build --tag sprise/chain-reader:0.4 -f chain-reader/Dockerfile -t chain-reader .;
docker build --tag sprise/indexer:0.4 -f indexer/Dockerfile -t indexer .;
docker build --tag sprise/code-compiler:0.2 -f code-compiler/Dockerfile -t code-compiler .;
docker build --tag sprise/dashboard:1.3  -t dashboard . --no-cache; # Need to run this in massbit/dashboard repo

docker push sprise/chain-reader:0.4;
docker push sprise/indexer:0.4;
docker push sprise/code-compiler:0.2;
docker push sprise/dashboard:1.3;  # Need to run this in massbit/dashboard repo