#!/bin/bash
SERVER=hughie@35.234.107.24
scp ../target/release/manager $SERVER:./massbitprotocol/e2e-test/manager
scp ../target/release/chain-reader $SERVER:./massbitprotocol/e2e-test/chain-reader