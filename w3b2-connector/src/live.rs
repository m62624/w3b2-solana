use crate::config::SyncConfig;
use crate::storage::Storage;
use anyhow::Result;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_client::rpc_response::Response;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio_stream::StreamExt;

/// live-sync worker: слушает новые блоки через WS
pub async fn run_live(cfg: SyncConfig, storage: Storage) -> Result<()> {
    // Сначала создаем экземпляр клиента
    let client = PubsubClient::new(&cfg.ws_url).await?;

    // Подписываемся на логи и получаем поток
    let (mut stream, _) = client
        .logs_subscribe(
            RpcTransactionLogsFilter::Mentions(vec![cfg.program_id.clone()]),
            RpcTransactionLogsConfig {
                commitment: Some(CommitmentConfig::confirmed()),
            },
        )
        .await?;

    while let Some(msg) = stream.next().await {
        let Response { context, value } = msg;
        let slot = context.slot;

        for log in value.logs {
            if log.contains(&cfg.program_id) {
                println!("[LIVE] slot={} log={}", slot, log);
            }
        }

        // Обновляем последний обработанный слот в хранилище
        storage.set_last_slot(slot);
    }

    Ok(())
}
