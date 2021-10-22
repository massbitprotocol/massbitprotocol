## Solana apis 
Diesel commands
```
diesel print-schema > src/schema.rs
diesel_ext -m > src/models.rs,
```
## Usage
Set environment variables: 
```
DATABASE_URL default value is: postgres://graph-node:PASSWORD@localhost/analytic
SOLANA_RPC_URL: default value is "http://194.163.156.242:8899"
API_ENDPOINT: ip:port on which server binds and listens for incoming requests, default value is "0.0.0.0:9090"
CONNECTION_POOL_SIZE: Database connection poolsize default 10
```
```shell
cargo run --bin solana-api
```
