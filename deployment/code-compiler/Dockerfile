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
RUN $HOME/.cargo/bin/rustup install 1.53.0
RUN $HOME/.cargo/bin/rustup default 1.53.0-x86_64-unknown-linux-gnu
RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain 1.53.0-x86_64-unknown-linux-gnu
RUN $HOME/.cargo/bin/rustup show

# Install NPM
RUN apt install -y npm
RUN curl -fsSL https://deb.nodesource.com/setup_14.x | bash -
RUN apt-get install -y nodejs

# Install python
RUN ls
RUN apt install -y python3.8 python3-pip wget unzip
RUN pip3 install -U Flask
RUN pip3 install -U flask-cors
RUN pip3 install -U ipfshttpclient
RUN pip3 install -U pyyaml
RUN ls -ll
COPY ./ massbitprotocol
RUN ls -ll

WORKDIR "massbitprotocol/code-compiler"
CMD python3 app.py