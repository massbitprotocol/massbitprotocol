FROM rust
COPY "target/release/indexer-api" .
RUN ls -ll
RUN ["chmod", "+x", "indexer-api"]
RUN ls -ll
CMD bash -c "RUST_LOG_TYPE=file ./indexer-api 2>&1 | tee console-indexer-api.log"