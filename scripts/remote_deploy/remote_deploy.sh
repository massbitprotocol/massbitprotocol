#!/bin/bash
cd ../..
cargo build --release
# Copy files to deploying machine
echo "Copy files ..."
rsync -avz ./target/release/chain-reader huy@34.159.170.173:./massbitprotocol/deployment/binary/
rsync -avz ./target/release/indexer-api huy@34.89.174.48:./massbitprotocol/deployment/binary/
rsync -avz ./target/release/massbit-graphql huy@34.89.174.48:./massbitprotocol/deployment/binary/
rsync -avz ./target/release/indexer-manager huy@34.159.83.129:./massbitprotocol/deployment/binary/

echo "Run deploy script"
ssh huy@34.159.170.173 < scripts/remote_deploy/deploy-chain-reader.sh
ssh huy@34.89.174.48 < scripts/remote_deploy/deploy-indexer-api-massbit-graphql.sh
ssh huy@34.159.83.129 < scripts/remote_deploy/deploy-indexer-manager.sh
