use crate::{
    events::{try_parse_log, EventSource},
    workers::synchronizer::WorkerContext,
};
use anyhow::Result;
use solana_client::{
    rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::RpcTransactionConfig,
    rpc_response::RpcConfirmedTransactionStatusWithSignature,
};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use solana_transaction_status::{UiTransactionEncoding, EncodedConfirmedTransactionWithStatusMeta};
use tokio::time::{sleep, Duration};

pub struct CatchupWorker {
    ctx: WorkerContext,
    program_id: solana_sdk::pubkey::Pubkey,
}

impl CatchupWorker {
    pub fn new(ctx: WorkerContext) -> Self {
        Self { ctx, program_id: w3b2_solana_program::ID }
    }

    pub async fn run(self) -> Result<()> {
        loop {
            tokio::select! {
                _ = sleep(Duration::from_secs(self.ctx.config.synchronizer.poll_interval_secs)) => {
                    if let Err(e) = self.sync_transactions().await {
                        tracing::error!("Error during catch-up sync: {}", e);
                    }
                },
                _ = self.ctx.dispatcher.command_tx.closed() => {
                    tracing::info!("CatchupWorker: shutdown signal received, exiting.");
                    return Ok(());
                }
            }
        }
    }

    async fn sync_transactions(&self) -> Result<()> {
        let signatures = self.fetch_new_signatures().await?;
        if !signatures.is_empty() {
            tracing::info!("Found {} new signatures to process.", signatures.len());
            self.process_signatures(signatures).await?;
        }
        Ok(())
    }

    async fn fetch_new_signatures(&self) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>> {
        let mut before: Option<Signature> = None;
        let last_known_sig = self.ctx.storage.get_last_sig().await?;
        let mut signatures_to_process = Vec::new();

        tracing::debug!("Starting catch-up from last known signature: {:?}", last_known_sig);

        loop {
            let page = self.fetch_signature_page(before).await?;
            if page.is_empty() {
                break;
            }
            before = page.last().and_then(|s| s.signature.parse().ok());

            if let Some(ref known_sig) = last_known_sig {
                if let Some(pos) = page.iter().position(|s| &s.signature == known_sig) {
                    signatures_to_process.extend_from_slice(&page[..pos]);
                    break;
                }
            }
            signatures_to_process.extend(page);
        }

        signatures_to_process.reverse();
        Ok(signatures_to_process)
    }

    async fn fetch_signature_page(&self, before: Option<Signature>) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>> {
        let config = GetConfirmedSignaturesForAddress2Config {
            before,
            until: None,
            limit: Some(self.ctx.config.synchronizer.max_signature_fetch),
            commitment: Some(CommitmentConfig { commitment: self.ctx.config.solana.commitment }),
        };
        self.ctx.rpc_client.get_signatures_for_address_with_config(&self.program_id, config).await.map_err(Into::into)
    }

    async fn process_signatures(&self, signatures: Vec<RpcConfirmedTransactionStatusWithSignature>) -> Result<()> {
        let current_slot = self.ctx.rpc_client.get_slot().await?;
        let max_depth = self.ctx.config.synchronizer.max_catchup_depth;

        for sig_info in signatures {
            if !self.is_within_catchup_depth(&sig_info, current_slot, max_depth) {
                continue;
            }
            if let Err(e) = self.process_one_transaction(&sig_info).await {
                tracing::error!(signature = %sig_info.signature, "Failed to process transaction: {}", e);
            }
        }

        Ok(())
    }

    fn is_within_catchup_depth(&self, sig_info: &RpcConfirmedTransactionStatusWithSignature, current_slot: u64, max_depth: Option<u64>) -> bool {
        if let Some(depth) = max_depth {
            if sig_info.slot < current_slot.saturating_sub(depth) {
                tracing::debug!("Skipping {} from slot {} due to max_catchup_depth", sig_info.signature, sig_info.slot);
                return false;
            }
        }
        true
    }

    async fn process_one_transaction(&self, sig_info: &RpcConfirmedTransactionStatusWithSignature) -> Result<()> {
        let sig = sig_info.signature.parse::<Signature>()?;
        if let Some(tx) = self.fetch_enriched_transaction(&sig).await? {
            if let Some(logs) = tx.transaction.meta.and_then(|meta| meta.log_messages.into()) {
                self.dispatch_events_from_logs(logs).await;
            }
            self.ctx.storage.set_sync_state(tx.slot, &sig_info.signature).await?;
        }
        Ok(())
    }

    async fn fetch_enriched_transaction(&self, sig: &Signature) -> Result<Option<EncodedConfirmedTransactionWithStatusMeta>> {
        let tx_config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            commitment: Some(CommitmentConfig { commitment: self.ctx.config.solana.commitment }),
            max_supported_transaction_version: Some(0),
        };
        match self.ctx.rpc_client.get_transaction_with_config(sig, tx_config).await {
            Ok(tx) => Ok(Some(tx)),
            Err(e) => {
                tracing::error!("Failed to get transaction {}: {}", sig, e);
                Ok(None)
            }
        }
    }

    async fn dispatch_events_from_logs(&self, logs: Vec<String>) {
        for log in logs {
            if let Ok(mut event) = try_parse_log(&log) {
                event.source = EventSource::Catchup;
                self.ctx.dispatcher.dispatch(event).await;
            }
        }
    }
}
