# Docker file for building code-compiler
FROM ubuntu:18.04

RUN DEBIAN_FRONTEND=noninteractive  apt update && \
apt install -y git curl && \
DEBIAN_FRONTEND=noninteractive curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
apt install -y cmake pkg-config libssl-dev git gcc build-essential clang libclang-dev libpq-dev \
    libssl-dev libudev-dev pkg-config zlib1g-dev llvm clang make && \


$HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain stable && \
$HOME/.cargo/bin/rustup toolchain install nightly-2021-05-20 && \
$HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain nightly-2021-05-20 && \
$HOME/.cargo/bin/rustup install 1.53.0 && \
$HOME/.cargo/bin/rustup default 1.53.0-x86_64-unknown-linux-gnu && \
$HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain 1.53.0-x86_64-unknown-linux-gnu && \
$HOME/.cargo/bin/rustup show && \

# Install NPM
apt install -y npm && \
curl -fsSL https://deb.nodesource.com/setup_14.x | bash - && \
apt-get install -y nodejs && \

# Install and upgrade to python 3.8
#RUN ls
apt install -y python3 && \
apt install -y python3.8 && \
rm /usr/bin/python3 && \
ln -s python3.8 /usr/bin/python3 && \

# Install python lib
apt install -y python3-pip wget unzip && \
pip3 install -U Flask && \
pip3 install -U flask-cors && \
pip3 install -U ipfshttpclient && \
    pip3 install -U pyyaml && \
    apt-get autoremove -y && \
        apt-get clean -y 

#RUN ls -ll
COPY ./ massbitprotocol
#RUN ls -ll

WORKDIR "massbitprotocol/code-compiler"
CMD python3 app.py
