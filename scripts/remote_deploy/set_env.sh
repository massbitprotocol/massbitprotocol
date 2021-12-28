#!/bin/bash
echo "Deploy indexer-api"
rsync -avz .bash_profile indexer-api:./
rsync -avz ../../apis/indexer-api/src/user_managerment/pubkey.pem indexer-api:./massbitprotocol/deployment/binary/
echo "Deploy indexer-manager"
rsync -avz .bash_profile indexer-manager:./
echo "Deploy chain-reader"
rsync -avz .bash_profile chain-reader:./

