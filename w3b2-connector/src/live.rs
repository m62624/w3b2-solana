use crate::config::SyncConfig;
use crate::events::{BridgeEvent, try_parse_log};
use crate::storage::Storage;
use anyhow::Result;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_client::rpc_response::Response;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

/// live-sync worker: слушает новые блоки через WS и отправляет события в канал
pub async fn run_live(
    cfg: SyncConfig,
    storage: Storage,
    sender: mpsc::Sender<BridgeEvent>,
) -> Result<()> {
    let client = PubsubClient::new(&cfg.ws_url).await?;

    let (mut stream, _) = client
        .logs_subscribe(
            RpcTransactionLogsFilter::Mentions(vec![cfg.program_id.clone()]),
            RpcTransactionLogsConfig {
                commitment: Some(CommitmentConfig::confirmed()),
            },
        )
        .await?;

    tracing::info!("Live worker connected to WebSocket and listening for logs.");

    while let Some(msg) = stream.next().await {
        let Response { context, value } = msg;
        let slot = context.slot;

        // Предотвращаем двойную обработку, если catch-up воркер еще не закончил
        if slot <= storage.get_last_slot() {
            tracing::trace!(
                "Live worker skipping slot {} as it is less than or equal to last processed slot {}.",
                slot,
                storage.get_last_slot()
            );
            continue;
        }

        for log in value.logs {
            if let Ok(ev) = try_parse_log(&log) {
                if !matches!(ev, BridgeEvent::Unknown) {
                    tracing::info!("[LIVE] slot={} event={:?}", slot, ev);
                    if sender.send(ev).await.is_err() {
                        return Err(anyhow::anyhow!(
                            "MPSC channel closed. Shutting down live worker."
                        ));
                    }
                }
            }
        }

        // Обновляем состояние
        storage.set_last_slot(slot);
        storage.set_last_sig(&value.signature);
    }

    Ok(())
}
