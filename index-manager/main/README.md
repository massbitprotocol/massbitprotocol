## Start with logging to file
```shell
RUST_LOG=info RUST_LOG_TYPE=file cargo run --bin index-manager-main
RUST_LOG=debug RUST_LOG_TYPE=file cargo run --bin index-manager-main
```

## Start with logging to console
```shell
cargo run --bin index-manager-main
RUST_LOG=debug cargo run --bin index-manager-main
```