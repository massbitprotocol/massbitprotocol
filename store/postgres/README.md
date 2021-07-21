## Graphql schema parser 
Clone from https://github.com/graphprotocol/graph-node/tree/v0.22.0/store/postgres

Modify file relational.rs to generate ddl as list of seperated queries and list table names
### Integration
Include protection against stack overflow when parsing from this PR: 

https://github.com/graphql-rust/graphql-parser/commit/45167b53e9533c331298683577ba8df7e43480ac

Add patch to workspace toml
```
[patch.crates-io]
graphql-parser = {git="https://github.com/graphql-rust/graphql-parser", rev="45167b53e9533c331298683577ba8df7e43480ac"}
```
Using
```
use store_postgres::layout;
..........................
    let schema : &str = r#"..."#
    let namespace = "01";
    let result = layout::gen_ddls(schema, namespace);
    match result {
        Ok(res) => {
            for ddl in &res.0 {
                println!("{}", ddl);
            }
            println!("Table names:");
            for table in &res.1 {
                println!("{},", table);
            }
        },
        Err(_) => println!("Invalid schema")
    }
```