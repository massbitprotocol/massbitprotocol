FROM rust
COPY . .
RUN ls -ll
RUN ["chmod", "+x", "target/release/chain-reader/chain-reader"]
RUN ls -ll
CMD bash -c "sleep 5; RUST_LOG=info ./target/release/chain-reader/chain-reader"
