pub const CARGO_TOML: &str = r#"
[package]
name = "block"
version = "0.1.0"
edition = "2018"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
diesel = { version = "1.4.0", features = ["postgres"] }
chrono = "0.4.19"
hex = "0.4.3"
anyhow = "1.0.44"
uuid = { version = "0.8", features = ["serde", "v4"] }
num-traits = "0.2.12"
arrayref = "0.3.6"
arbitrary = { version = "0.4.6", features = ["derive"], optional = true }
bincode = "1.3.1"
enumflags2 = "0.6.4"
log = "0.4.14"
num_enum = "0.5.0"
thiserror = "1.0.20"
safe-transmute = "0.11.0"
lazy_static     = "1.4.0"
serde = "1.0.114"
serde_json = "1.0.69"
static_assertions = "1.1.0"
spl-token = { version = "3.0.0-pre1", features = ["no-entrypoint"] }

# Massbit dependencies
[dependencies.massbit-solana-sdk]
package = "massbit-solana-sdk"
git = "https://github.com/massbitprotocol/massbitprotocol.git"
branch = "main"

[dependencies.solana-transaction-status]
package = "solana-transaction-status"
git = "https://github.com/massbitprotocol/solana.git"
branch = "massbit"

[dependencies.solana-account-decoder]
package = "solana-account-decoder"
git = "https://github.com/massbitprotocol/solana.git"
branch = "massbit"


[dependencies.solana-client]
package = "solana-client"
git = "https://github.com/massbitprotocol/solana.git"
branch = "massbit"

[dependencies.solana-sdk]
package = "solana-sdk"
git = "https://github.com/massbitprotocol/solana.git"
branch = "massbit"

[dependencies.solana-program]
package = "solana-program"
git = "https://github.com/massbitprotocol/solana.git"
branch = "massbit"

[lib]
crate-type = ["cdylib"]

[workspace]

"#;

pub const INDEXER_YAML: &str = r#"
specVersion: 0.0.2
description: Indexer for Serum
repository: https://github.com/massbitprotocol/massbitprotocol/tree/main/user-example
schema:
  file: ./schema.graphql
dataSources:
  - kind: solana
    name: {{name}}
    network: mainnet
    source:
      address: {{address}}
      abi: Serum,
      start_block: {{start_block}}
    mapping:
      kind: solana/BlockHandler
      apiVersion: 0.0.4
      language: rust
      entities:
        - Serum
      handlers:
        - handler: handleBlock
          kind: solana/BlockHandler
      file: ./src/mapping.rs
      abis:
        - name: Serum
          file: ./abis/Serum.json
"#;
