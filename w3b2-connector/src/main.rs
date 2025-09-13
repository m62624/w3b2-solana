mod catchup;
mod config;
mod live;
mod storage;

use anyhow::Result;
use tokio::task;

use catchup::run_catchup;
use config::SyncConfig;
use live::run_live;
use storage::Storage;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = SyncConfig::default();
    let storage = Storage::new("target/sync.db")?;

    let s1 = storage.clone();
    let s2 = storage;

    // Catch-up
    task::spawn(async move {
        if let Err(e) = run_catchup(cfg.clone(), s1).await {
            eprintln!("Catch-up error: {:?}", e);
        }
    });

    // Live sync
    run_live(cfg, s2).await?;

    Ok(())
}
