use anyhow::Result;
use chain_reader::command;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    command::run().await
}
