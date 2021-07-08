mod command;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    command::run().await

}

