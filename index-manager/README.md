## Prerequisites
The store (s3 / ipfs / local) should contain 
- a config file (project.yaml)

```yaml
schema:
  file: ./schema.graphql

dataSources:
  - kind: substrate
    name: Index
    network: https://data-seed-prebsc-1-s1.binance.org:8545/
    mapping:
      language: rust
      handlers:
        - handler: handleBlock
          kind: substrate/BlockHandler
        - handler: handleCall
          kind: substrate/CallHandler
        - handler: handleEvent
          kind: substrate/EventHandler
```

- a schema (schema.graphql / rust model)
```
type IndexSchema @entity{
  id: ID!
  account: BigInt!
  date: Date!
}
```

- a mapping file (mapping.rs)
```
use SubstrateBlock;
use SubstrateEvent;
use SubstrateCall;
use std::time::{SystemTime, UNIX_EPOCH};

fn handleBlock(block: SubstrateBlock) {
    // Add user logic here
    block.date = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    block.save();
}

fn handleEvent(event: SubstrateEvent) {
    // Add user logic here
    event.date = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    event.save();
}

fn handleCall(call: SubstrateCall) {
    // Add user logic here
    call.date = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    call.save();
}
```

## API

Method: index_deploy

Create new index (update on: 25-6-2021) 

Description: 
- For every new request, indexer will start a new thread to index data.
- params: 
  - index_config_url: the location of (mapping, schema, config file) so the index manager can query.

```http request
curl --location --request POST 'SERVER_ADDRESS:SERVER_PORT' \
--header 'Content-Type: application/json' \
--data-raw '{"jsonrpc": "2.0", "method": "index_deploy", "id":123 }'
```