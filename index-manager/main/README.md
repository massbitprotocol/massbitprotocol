# Index Manager

## Default config (If ENV not defined)
- Rust log type default is CONSOLE (updated on: 13-8-2021)
- Rust log level default is INFO (updated on: 13-8-2021)
- Index Manager won't automatically restart the index (updated on: 13-8-2021)

## Start with logging to file
```shell
RUST_LOG_TYPE=file cargo run --bin index-manager-main
RUST_LOG=debug RUST_LOG_TYPE=file cargo run --bin index-manager-main
```

## Start with logging to console
```shell
cargo run --bin index-manager-main
RUST_LOG=debug cargo run --bin index-manager-main
```

## Automatically restart all the index
```shell
INDEX_MANAGER_RESTART_INDEX=true cargo run --bin index-manager-main
```
