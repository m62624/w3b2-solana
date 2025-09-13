use crate::config::SyncConfig;
use crate::events::BridgeEvent;
use crate::storage::Storage;
use crate::{catchup, live};
use anyhow::Result;
use tokio::sync::mpsc;
use tokio_stream::{Stream, wrappers::ReceiverStream};

pub struct Synchronizer;

impl Synchronizer {
    pub fn builder() -> SynchronizerBuilder {
        SynchronizerBuilder::default()
    }
}

#[derive(Default)]
pub struct SynchronizerBuilder {
    config: Option<SyncConfig>,
    storage: Option<Storage>,
}

impl SynchronizerBuilder {
    pub fn with_config(mut self, config: SyncConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_storage(mut self, storage: Storage) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Задает максимальную глубину поиска в блоках для catch-up воркера.
    /// Переопределяет значение из `SyncConfig`.
    pub fn with_max_catchup_depth(mut self, depth: u64) -> Self {
        if let Some(ref mut cfg) = self.config {
            cfg.max_catchup_depth = Some(depth);
        }
        self
    }

    /// Запускает оба воркера (catch-up и live) и возвращает единый поток событий.
    pub async fn start(self) -> Result<impl Stream<Item = BridgeEvent>> {
        let config = self
            .config
            .ok_or_else(|| anyhow::anyhow!("Config not provided"))?;
        let storage = self
            .storage
            .ok_or_else(|| anyhow::anyhow!("Storage not provided"))?;

        // Создаем MPSC канал для объединения событий из обоих воркеров
        let (tx, rx) = mpsc::channel(100);

        // Запускаем catch-up воркер в отдельной задаче
        let catchup_cfg = config.clone();
        let catchup_storage = storage.clone();
        let catchup_tx = tx.clone();
        tokio::spawn(async move {
            if let Err(e) = catchup::run_catchup(catchup_cfg, catchup_storage, catchup_tx).await {
                tracing::error!("Catch-up worker failed: {}", e);
            }
        });

        // Запускаем live воркер в отдельной задаче
        let live_cfg = config;
        let live_storage = storage;
        let live_tx = tx;
        tokio::spawn(async move {
            if let Err(e) = live::run_live(live_cfg, live_storage, live_tx).await {
                tracing::error!("Live worker failed: {}", e);
            }
        });

        // Возвращаем приемник канала, обернутый в Stream
        Ok(ReceiverStream::new(rx))
    }
}
