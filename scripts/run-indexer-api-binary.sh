#!/bin/sh
RUST_LOG_TYPE=file RUST_LOG=debug ./deployment/binary/indexer-api 2>&1 | tee log/console-indexer-api.log