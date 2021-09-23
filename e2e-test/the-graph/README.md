# The Graph
## Prerequisites
```shell
make init-docker
make init-npm
```

## Setting up the graph to test data integrity
Setup docker for the-graph
```shell
git clone https://github.com/graphprotocol/graph-node/
cd graph-node/docker
sudo apt install -y jq
 [log out of session]
 [re-login to new session]
./setup.sh
```

Update graph-node/docker/docker-compose.yml
```yaml
version: '3'
services:
  graph-node:
    image: graphprotocol/graph-node
    ports:
      - '8000:8000'
      - '8001:8001'
      - '8020:8020'
      - '8030:8030'
      - '8040:8040'
    depends_on:
      - ipfs
      - postgres
    environment:
      postgres_host: postgres
      postgres_user: graph-node
      postgres_pass: let-me-in
      postgres_db: graph-node
      ipfs: 'ipfs:5001'
      ethereum: 'matic:https://polygon-rpc.com/'
      GRAPH_LOG: info
  ipfs:
    image: ipfs/go-ipfs:v0.4.23
    ports:
      - '5002:5001'
    volumes:
      - ./data/ipfs-the-graph:/data/ipfs
  postgres:
    image: postgres
    ports:
      - '5433:5432'
    command: ["postgres", "-cshared_preload_libraries=pg_stat_statements"]
    environment:
      POSTGRES_USER: graph-node
      POSTGRES_PASSWORD: let-me-in
      POSTGRES_DB: graph-node
    volumes:
      - ./data/postgres-the-graph:/var/lib/postgresql/data
```

Start the-graph service
```shell
docker-compose up -d
```

Start new index with the graph
```shell
cd massbitprotocol/user-example/polygon/wasm/quickswap
npm install
npm run codegen
npm run build
npm run create-local
[Fix package.json to `deploy-local` to point to IPFS 5002]
npm run deploy-local
```

Or with https://github.com/QuickSwap/QuickSwap-subgraph
```shell
git clone https://github.com/QuickSwap/QuickSwap-subgraph
cd QuickSwap-subgraph
npm install
npm run codegen
npm run create-local
[Fix package.json to `deploy-local` to point to IPFS 5002]
npm run deploy-local
```
