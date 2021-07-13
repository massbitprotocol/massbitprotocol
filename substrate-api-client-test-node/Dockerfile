# Docker file for running substrate-api-client-test-node
FROM ubuntu:18.04

RUN apt update
RUN apt install -y git curl
RUN git clone https://github.com/scs/substrate-api-client-test-node
WORKDIR "substrate-api-client-test-node"
RUN rm -f rust-toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN apt install -y cmake pkg-config libssl-dev git gcc build-essential clang libclang-dev
RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain stable

RUN $HOME/.cargo/bin/rustup toolchain install nightly-2021-05-20
RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain nightly-2021-05-20
RUN $HOME/.cargo/bin/cargo run -- --dev
