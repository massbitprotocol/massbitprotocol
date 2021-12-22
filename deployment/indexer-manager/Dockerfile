FROM rust
COPY "target/release/indexer-manager" .
RUN ls -ll
RUN ["chmod", "+x", "indexer-manager"]
RUN ls -ll
CMD bash -c "RUST_LOG_TYPE=file ./indexer-manager 2>&1 | tee console-indexer-manager.log"