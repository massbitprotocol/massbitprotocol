# Docker file for building code-compiler
FROM ubuntu:18.04

RUN apt update
RUN apt install -y git curl

# Installing Rust and Cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN apt install -y cmake pkg-config libssl-dev git gcc build-essential clang libclang-dev libpq-dev
RUN apt-get update
RUN apt-get install -y libssl-dev libudev-dev pkg-config zlib1g-dev llvm clang make

RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain stable
RUN $HOME/.cargo/bin/rustup toolchain install nightly-2021-05-20
RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain nightly-2021-05-20

# Code compiler
COPY ./ massbitprotocol
WORKDIR "massbitprotocol/code-compiler"

RUN ls
RUN apt install -y python3
RUN apt install -y python3-pip
RUN pip3 install -U Flask
RUN pip3 install -U flask-cors
RUN pip3 install -U ipfshttpclient
CMD python3 app.py