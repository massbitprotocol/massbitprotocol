# Merge all services into one docker image because we don't support calling services by their network-name yet
FROM ubuntu:18.04

# Core config
RUN apt update
RUN apt install -y git curl
COPY ./ massbitprotocol

# Setup Cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN apt install -y cmake pkg-config libssl-dev git gcc build-essential clang libclang-dev libpq-dev
RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain stable
RUN $HOME/.cargo/bin/rustup toolchain install nightly-2021-05-20
RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain nightly-2021-05-20

# Docker file for building code-compiler
WORKDIR "massbitprotocol/code-compiler"  
RUN ls -ll
RUN ls
RUN apt install -y python3
RUN apt install -y python3-pip
RUN pip3 install -U Flask
RUN pip3 install -U flask-cors
RUN pip3 install -U ipfshttpclient

# Building chain-reader & index-manager
WORKDIR "massbitprotocol"
RUN ls -ll
RUN $HOME/.cargo/bin/cargo build --release

# Script to run all the services
WORKDIR "massbitprotocol" 
RUN ls -ll

WORKDIR "/" 
RUN ls -ll


CMD ./wrapper_script.sh