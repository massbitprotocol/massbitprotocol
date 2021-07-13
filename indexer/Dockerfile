# Docker file for running index-manager
FROM ubuntu:18.04

RUN apt update
RUN apt install -y git curl
RUN git clone --branch docker-for-integrate-with-vu https://github.com/massbitprotocol/massbitprotocol
WORKDIR "massbitprotocol"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN apt install -y cmake pkg-config libssl-dev git gcc build-essential clang libclang-dev
RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain stable

# Refactor when move location
# COPY Cargo.lock .

RUN $HOME/.cargo/bin/rustup toolchain install nightly-2021-05-20
RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain nightly-2021-05-20
RUN $HOME/.cargo/bin/cargo run --bin index-manager-main