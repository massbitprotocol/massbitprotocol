#!/bin/bash
echo "Run deploy script"
rsync -avz .bash_profile indexer-api:./
rsync -avz .bash_profile indexer-manager:./
rsync -avz .bash_profile chain-reader:./

