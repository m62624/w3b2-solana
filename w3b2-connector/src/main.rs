use anyhow::Result;
use tokio::task;

#[tokio::main]
async fn main() -> Result<()> {
    // let cfg = SyncConfig::default();
    // let storage = Storage::new("target/sync.db")?;

    // let catchup_storage = storage.clone();
    // let live_storage = storage;

    // let catchup_task = task::spawn(async move {
    //     if let Err(e) = run_catchup(cfg.clone(), catchup_storage).await {
    //         eprintln!("Catch-up error: {:?}", e);
    //     }
    // });

    // let live_task = task::spawn(async move {
    //     if let Err(e) = run_live(cfg, live_storage).await {
    //         eprintln!("Live sync error: {:?}", e);
    //     }
    // });

    // tokio::try_join!(catchup_task, live_task)?;

    Ok(())
}
