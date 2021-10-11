## Analytic pipeline 

## Usage
Set environment variable DATABASE_URL, default value is: postgres://graph-node:PASSWORD@localhost/analytic
```shell
cargo run --bin analytics -- -c ethereum -n matic -b 15000000
cargo run --bin analytics -- -c solana -n mainnet -b 80000000
```
