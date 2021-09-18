#!/bin/sh
RUST_LOG_TYPE=file cargo run --bin index-manager-main 2>&1 | tee log/console-index-manager.log