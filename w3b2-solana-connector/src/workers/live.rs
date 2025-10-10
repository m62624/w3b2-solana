use crate::{
    events::{try_parse_log, BridgeEvent, EventSource},
    workers::synchronizer::WorkerContext,
};
use anyhow::Result;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_response::{Response, RpcLogsResponse},
};
use solana_sdk::commitment_config::CommitmentConfig;
use tokio_stream::StreamExt;

pub struct LiveWorker {
    ctx: WorkerContext,
}

impl LiveWorker {
    pub fn new(ctx: WorkerContext) -> Self {
        Self { ctx }
    }

    pub async fn run(self) -> Result<()> {
        let client = PubsubClient::new(&self.ctx.config.solana.ws_url).await?;
        let (mut stream, _) = client
            .logs_subscribe(
                RpcTransactionLogsFilter::Mentions(vec![w3b2_solana_program::ID.to_string()]),
                RpcTransactionLogsConfig {
                    commitment: Some(CommitmentConfig { commitment: self.ctx.config.solana.commitment }),
                },
            )
            .await?;

        tracing::info!("Live worker connected to WebSocket, listening for logs...");

        loop {
            tokio::select! {
                Some(msg) = stream.next() => {
                    if let Err(e) = self.handle_log_message(msg).await {
                        tracing::error!("Error handling log message: {}", e);
                    }
                },
                _ = self.ctx.dispatcher.command_tx.closed() => {
                    tracing::info!("LiveWorker: shutdown signal received, exiting.");
                    return Ok(());
                },
                else => break,
            }
        }
        Ok(())
    }

    async fn handle_log_message(&self, msg: Response<RpcLogsResponse>) -> Result<()> {
        let Response { context, value } = msg;
        let slot = context.slot;

        if slot <= self.ctx.storage.get_last_slot().await? {
            return Ok(());
        }

        let events_to_dispatch: Vec<BridgeEvent> = value
            .logs
            .into_iter()
            .filter_map(|log| try_parse_log(&log).ok())
            .map(|mut event| {
                event.source = EventSource::Live;
                tracing::info!("[LIVE] slot={} event={:?}", slot, &event);
                event
            })
            .collect();

        for event in events_to_dispatch {
            self.ctx.dispatcher.dispatch(event).await;
        }

        self.ctx.storage.set_sync_state(slot, &value.signature).await?;
        Ok(())
    }
}
