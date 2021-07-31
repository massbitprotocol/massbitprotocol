#!/bin/bash

curl https://sh.rustup.rs -sSf | sh
source $HOME/.cargo/env
rustup component add rustfmt

# For chain-reader + indexer manager
rustup update
sudo apt-get update
sudo apt-get install libssl-dev libudev-dev pkg-config zlib1g-dev llvm clang make

# For code compiler
sudo apt install -y python3 python3-pip wget unzip;
pip3 install -U Flask;
pip3 install -U flask-cors;
pip3 install -U ipfshttpclient;
