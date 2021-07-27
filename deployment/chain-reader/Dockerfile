FROM rust
COPY . .
RUN ls -ll
RUN ["chmod", "+x", "target/release/chain-reader/chain-reader"]
RUN ls -ll
CMD RUST_LOG=info ./target/release/chain-reader/chain-reader
