#!/bin/sh
RUST_LOG_TYPE=file RUST_LOG=debug ./deployment/binary/chain-reader 2>&1 | tee log/console-chain-reader.log