FROM rust
COPY . .
RUN ls -ll
RUN ["chmod", "+x", "target/release/index-manager-main/index-manager-main"]
RUN ls -ll
CMD ./target/release/index-manager-main/index-manager-main