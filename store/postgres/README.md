## Graphql schema parser 
Clone from https://github.com/graphprotocol/graph-node/tree/v0.22.0/store/postgres

Modify file relational.rs to generate ddl as list of seperated queries and list table names
# Migration CLI

## Usage
```shell
cargo run -- ddlgen -s schema.graphql -c project1.yaml -o ./migrations -h sessionid
```

## Input templates
`schema.graphql`
```graphql
type Block @entity {
    id: ID!
    block_number: Int!
    block_hash: String!
    sum_fee: Int!
    transaction_number: Int!
    success_rate: BigDecimal!
}

type Transaction @entity {
    id: ID!
    signature: String!
    timestamp: Int!
    fee: Int!
    block: Block!
    block_number: Int!
    success: Boolean!
}


type TransactionAccount @entity {
    id: ID!
    pub_key: String!
    pos_balance: Int!
    change_balance: Int!
    is_program: Boolean!
    transaction_own: Transaction!
    inner_account_index: Int!
}

type InstructionDetail @entity {
    id: ID!
    name: String
    is_decoded: Boolean!
}

type TransactionInstruction @entity {
    id: ID!
    transaction_own: Transaction!
    inner_account_index: Int!
    instruction_detail: InstructionDetail!

}

```
`project.yaml`
```yaml

database:
  catalog: graph-node
```
### Output
3 Files up.sql, down.sql, hasura_queries.json in {output}/{timestamp}_{sessionId}


`hasura_queries.json`
```yaml
{
  "type": "bulk",
  "args": [
    { "args": { "name": "TransactionInstruction","schema": "public" },"type": "track_table" },
    { "args": { "name": "Block","schema": "public" },"type": "track_table" },
    { "args": { "name": "Transaction","schema": "public" },"type": "track_table" },
    { "args": { "name": "TransactionAccount","schema": "public" },"type": "track_table" },
    { "args": { "name": "InstructionDetail","schema": "public" },"type": "track_table" }
  ]
}
```