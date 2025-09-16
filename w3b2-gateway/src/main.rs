use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Call the main application logic from the library crate.
    w3b2_gateway::run().await?;
    Ok(())
}
