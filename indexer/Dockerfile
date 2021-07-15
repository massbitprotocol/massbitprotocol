# Docker file for running index-manager
FROM ubuntu:18.04

RUN apt update
RUN apt install -y git curl

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN apt install -y cmake pkg-config libssl-dev git gcc build-essential clang libclang-dev libpq-dev
RUN apt-get update
RUN apt-get install -y libssl-dev libudev-dev pkg-config zlib1g-dev llvm clang make
RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain stable

# Use this when we have fixed importing solana error
#COPY ./ massbitprotocol

# We need to import solana like this because of some error with Cargo dependencies
COPY ./massbitprotocol/ massbitprotocol
COPY ./solana/ solana
WORKDIR "massbitprotocol" 

RUN $HOME/.cargo/bin/rustup toolchain install nightly-2021-05-20
RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain nightly-2021-05-20
RUN $HOME/.cargo/bin/cargo build --release
CMD ./target/release/index-manager-main