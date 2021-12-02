#!/bin/sh
./deployment/binary/manager --ipfs 127.0.0.1:5001 --config manager/config.toml 2>&1 | tee log/console-indexer-eth.log