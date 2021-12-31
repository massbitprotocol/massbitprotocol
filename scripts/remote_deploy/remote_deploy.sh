#!/bin/bash
cd ../..
cargo build --release
# Copy files to deploying machine
echo "Copy files ..."
rsync -avz ./target/release/chain-reader $1@chain-reader:./massbitprotocol/deployment/binary/
rsync -avz ./target/release/indexer-api $1@indexer-api:./massbitprotocol/deployment/binary/
rsync -avz ./target/release/massbit-graphql $1@indexer-api:./massbitprotocol/deployment/binary/
rsync -avz ./target/release/indexer-manager $1@indexer-manager:./massbitprotocol/deployment/binary/

echo "Run deploy script"
ssh $1@chain-reader < scripts/remote_deploy/deploy-chain-reader.sh
ssh $1@indexer-api < scripts/remote_deploy/deploy-indexer-api-massbit-graphql.sh
ssh $1@indexer-manager < scripts/remote_deploy/deploy-indexer-manager.sh
