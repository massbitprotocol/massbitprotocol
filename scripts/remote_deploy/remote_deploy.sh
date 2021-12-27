#!/bin/bash
cd ../..
cargo build --release
# Copy files to deploying machine
echo "Copy files ..."
rsync -avz ./target/release/chain-reader chain-reader:./massbitprotocol/deployment/binary/
rsync -avz ./target/release/indexer-api indexer-api:./massbitprotocol/deployment/binary/
rsync -avz ./target/release/massbit-graphql indexer-api:./massbitprotocol/deployment/binary/
rsync -avz ./target/release/indexer-manager indexer-manager:./massbitprotocol/deployment/binary/

echo "Run deploy script"
ssh chain-reader < scripts/remote_deploy/deploy-chain-reader.sh
ssh indexer-api < scripts/remote_deploy/deploy-indexer-api-massbit-graphql.sh
ssh indexer-manager < scripts/remote_deploy/deploy-indexer-manager.sh
