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

RUN ls
RUN apt install -y python3 python3-pip wget unzip
RUN pip3 install -U Flask
RUN pip3 install -U flask-cors
RUN pip3 install -U ipfshttpclient
RUN wget https://code-compiler-built-target.s3.ap-southeast-2.amazonaws.com/target.zip

COPY ./ massbitprotocol
RUN unzip target.zip -d massbitprotocol/code-compiler/stub/solana
RUN unzip target.zip -d massbitprotocol/code-compiler/stub/substrate

WORKDIR "massbitprotocol/code-compiler"

CMD python3 app.py