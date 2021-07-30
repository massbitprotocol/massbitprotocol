# Docker file for running chain-reader
FROM ubuntu:18.04

RUN apt update
RUN apt install -y git curl

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN apt install -y cmake pkg-config libssl-dev git gcc build-essential clang libclang-dev libpq-dev
RUN apt-get update
RUN apt-get install -y libssl-dev libudev-dev pkg-config zlib1g-dev llvm clang make

RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain stable

COPY ./ massbitprotocol
WORKDIR "massbitprotocol"

RUN $HOME/.cargo/bin/rustup toolchain install nightly-2021-05-20
RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain nightly-2021-05-20
RUN $HOME/.cargo/bin/cargo build --release

EXPOSE 50051
CMD bash -c "sleep 5; RUST_LOG=debug ./target/release/chain-reader"
