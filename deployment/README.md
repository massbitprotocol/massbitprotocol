## Build docker
```bash
docker build -t chain-reader:0.1 -f ./deployment/chain-reader/Dockerfile .
docker build -t indexer-manager:0.1 -f ./deployment/indexer-manager/Dockerfile .
docker build -t indexer-api:0.1 -f ./deployment/indexer-api/Dockerfile .
```
## Run docker 
```bash
docker run -d -t -i -p 50051:50051 -e SOLANA_URL=https://solana-api.projectserum.com  --name chain-reader chain-reader:0.1
docker run --network="host" -d -t -i -p 3032:3032 --name indexer-manager indexer-manager:0.1 
docker run --network="host" -d -t -i -p 3031:3031 --name indexer-api indexer-api:0.1 
```
## Check logs
```
docker logs indexer-manager -f
```

## Remove docker
```
docker container rm chain-reader -f
```




