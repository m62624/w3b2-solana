use crate::events::EventSource;
use crate::workers::synchronizer::WorkerContext;
use anyhow::Result;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_response::Response,
};
use solana_sdk::commitment_config::CommitmentConfig;
use tokio_stream::StreamExt;

/// A background worker that provides real-time event synchronization.
///
/// The `LiveWorker` connects to a Solana cluster's WebSocket endpoint and subscribes
/// to logs that mention the W3B2 Bridge program ID. It parses these logs in real-time,
/// converts them into `BridgeEvent`s, and broadcasts them to the rest of the system.
/// It works in tandem with the `CatchupWorker` to provide a complete and resilient
/// event stream.
pub struct LiveWorker {
    ctx: WorkerContext,
}

impl LiveWorker {
    /// Creates a new `LiveWorker`.
    pub fn new(ctx: WorkerContext) -> Self {
        Self { ctx }
    }

    /// Runs the main loop for the live worker.
    ///
    /// This method establishes a WebSocket connection to the Solana cluster and
    /// subscribes to transaction logs mentioning the program ID. It then enters
    /// an infinite loop to process incoming log messages, parse them into events,
    /// and broadcast them. The loop also handles graceful shutdown when the main
    /// event channel is closed.
    pub async fn run(self) -> Result<()> {
        let client = PubsubClient::new(&self.ctx.config.solana.ws_url).await?;

        let (mut stream, _) = client
            .logs_subscribe(
                RpcTransactionLogsFilter::Mentions(vec![w3b2_program::ID.to_string()]),
                RpcTransactionLogsConfig {
                    commitment: Some(CommitmentConfig {
                        commitment: self.ctx.config.solana.commitment,
                    }),
                },
            )
            .await?;

        tracing::info!("Live worker connected to WebSocket and listening for logs.");

        loop {
            tokio::select! {
                Some(msg) = stream.next() => {
                    let Response { context, value } = msg;
                    let slot = context.slot;

                    // To prevent processing duplicate events that might have been
                    // handled by the CatchupWorker just before the live event arrived,
                    // we check if the slot of the incoming log is newer than the last
                    // slot we've persisted.
                    if slot <= self.ctx.storage.get_last_slot().await? {
                        continue;
                    }

                    for log in value.logs {
                        if let Ok(mut event) = crate::events::try_parse_log(&log) {
                            tracing::info!("[LIVE] slot={} event={:?}", slot, &event);
                            event.source = EventSource::Live;
                            self.ctx.dispatcher.dispatch(event).await;
                        }
                    }

                    self.ctx
                        .storage
                        .set_sync_state(slot, &value.signature)
                        .await?;
                },
                // If the main broadcast channel is closed, it's a signal from the
                // EventManager to shut down gracefully.
                _ = self.ctx.dispatcher.command_tx.closed() => {
                    tracing::info!("LiveWorker: event channel closed, shutting down.");
                    return Ok(());
                },
                else => break,
            }
        }
        Ok(())
    }
}
