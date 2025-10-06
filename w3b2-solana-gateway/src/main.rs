use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    w3b2_solana_gateway::run().await?;
    Ok(())
}
