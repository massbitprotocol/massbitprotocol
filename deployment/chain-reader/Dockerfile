FROM rust
COPY "target/release/chain-reader" .
RUN ls -ll
RUN ["chmod", "+x", "chain-reader"]
RUN ls -ll
CMD bash -c "RUST_LOG_TYPE=file ./chain-reader 2>&1 | tee console-chain-reader.log"