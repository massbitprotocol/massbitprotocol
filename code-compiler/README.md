## Code Compiler Server


## API
Endpoint: /compile
- Method: Post
- Description: to make a new a request building a SO file, we need the URL encoded data of:
  - models.rs (created with diesel CLI)
  - schema.rs (created with diesel CLI)
  - up.sql (created with diesel CLI)
  - project.yaml
  - SO.file (created after run cargo build)
- Payload:
```json
{
    "mapping.rs": "use+super%3A%3Amodels%3A%3ANewBlock%3B%0D%0Ause+super%3A%3Aschema%3A%3Ablocks%3B%0D%0Ause+diesel%3A%3Apg%3A%3APgConnection%3B%0D%0Ause+diesel%3A%3Aprelude%3A%3A%2A%3B%0D%0Ause+massbit_chain_substrate%3A%3Adata_type%3A%3ASubstrateBlock%3B%0D%0Ause+plugin%3A%3Acore%3A%3A%7BBlockHandler%2C+InvocationError%7D%3B%0D%0Ause+std%3A%3Aenv%3B%0D%0Ause+index_store%3A%3Acore%3A%3AIndexStore%3B%0D%0A%0D%0A%23%5Bderive%28Debug%2C+Clone%2C+PartialEq%29%5D%0D%0Apub+struct+BlockIndexer%3B%0D%0A%0D%0Aimpl+BlockHandler+for+BlockIndexer+%7B%0D%0A++++fn+handle_block%28%26self%2C+store%3A+%26IndexStore%2C+substrate_block%3A+%26SubstrateBlock%29+-%3E+Result%3C%28%29%2C+InvocationError%3E+%7B%0D%0A++++++++println%21%28%22%5B.SO+File%5D+triggered%21%22%29%3B%0D%0A%0D%0A++++++++let+number+%3D+substrate_block.header.number+as+i64%3B%0D%0A++++++++let+new_block+%3D+NewBlock+%7B+number+%7D%3B%0D%0A%0D%0A++++++++store.save%28blocks%3A%3Atable%2C+new_block%29%3B%0D%0A++++++++Ok%28%28%29%29%0D%0A++++%7D%0D%0A%7D%0D%0A",
    "models.rs": "use+super%3A%3Aschema%3A%3Ablocks%3B%0D%0Ause+diesel%3A%3A%7BPgConnection%2C+Connection%2C+RunQueryDsl%7D%3B%0D%0A%0D%0A%23%5Bderive%28Insertable%29%5D%0D%0A%23%5Btable_name+%3D+%22blocks%22%5D%0D%0Apub+struct+NewBlock+%7B%0D%0A++++pub+number%3A+i64%2C%0D%0A%7D",
    "schema.rs": "table%21+%7B%0D%0A++++blocks+%28id%29+%7B%0D%0A++++++++id+-%3E+Int4%2C%0D%0A++++++++number+-%3E+Int8%2C%0D%0A++++%7D%0D%0A%7D%0D%0A",
    "project.yaml": "schema%3A%0D%0A++file%3A+.%2Fschema.graphql%0D%0A%0D%0AdataSources%3A%0D%0A++-+kind%3A+substrate%0D%0A++++name%3A+Index%0D%0A++++network%3A+https%3A%2F%2Fdata-seed-prebsc-1-s1.binance.org%3A8545%2F%0D%0A++++mapping%3A%0D%0A++++++language%3A+rust%0D%0A++++++handlers%3A%0D%0A++++++++-+handler%3A+handleBlock%0D%0A++++++++++kind%3A+substrate%2FBlockHandler%0D%0A++++++++-+handler%3A+handleCall%0D%0A++++++++++kind%3A+substrate%2FCallHandler%0D%0A++++++++-+handler%3A+handleEvent%0D%0A++++++++++kind%3A+substrate%2FEventHandler",
    "up.sql": "CREATE+TABLE+blocks+%28%0D%0A+++id+SERIAL+PRIMARY+KEY%2C%0D%0A+++number+BIGINT+NOT+NULL%0D%0A%29"
}
```