use anyhow::Result;
use tokio_stream::StreamExt;
use w3b2_connector::Storage;
use w3b2_connector::SyncConfig;
use w3b2_connector::Synchronizer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    // Инициализируем логгер для отладки

    // В реальном приложении конфигурация будет загружаться из файла или переменных окружения
    let config = SyncConfig::default();
    let storage = Storage::new("./w3b2_db")?;

    tracing::info!("Starting synchronizer for program: {}", config.program_id);

    // По умолчанию глубина поиска не ограничена, но ее можно задать через `with_max_catchup_depth`.
    let mut event_stream = Synchronizer::builder()
        .with_config(config)
        .with_storage(storage)
        // .with_max_catchup_depth(10_000_000) // Пример установки ограничения
        .start()
        .await?;

    tracing::info!("Synchronizer started. Listening for events...");

    while let Some(event) = event_stream.next().await {
        tracing::info!("[MAIN] Received event: {:?}", event);
    }

    Ok(())
}
