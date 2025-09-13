use anyhow::Result;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::signature::Signature;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

use crate::config::SyncConfig;
use crate::storage::Storage;
use crate::{events::try_parse_log, events::BridgeEvent};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature;
use solana_transaction_status::UiTransactionEncoding;

const SIGNATURE_FETCH_LIMIT: usize = 1000;

/// catch-up worker
pub async fn run_catchup(
    cfg: SyncConfig,
    storage: Storage,
    sender: mpsc::Sender<BridgeEvent>,
) -> Result<()> {
    let client = RpcClient::new(cfg.rpc_url.clone());
    let program_id: solana_sdk::pubkey::Pubkey = w3b2_bridge_program::ID;

    loop {
        let mut before_sig: Option<Signature> = None;
        let last_known_sig_str = storage.get_last_sig();
        let mut signatures_to_process: Vec<RpcConfirmedTransactionStatusWithSignature> = Vec::new();
        let mut fetched_count = 0;

        tracing::info!(
            "Starting catch-up from last known signature: {:?}",
            last_known_sig_str
        );

        // постраничная загрузка
        'fetch_loop: loop {
            let sigs = client
                .get_signatures_for_address_with_config(
                    &program_id,
                    GetConfirmedSignaturesForAddress2Config {
                        before: before_sig,
                        until: None,
                        limit: Some(SIGNATURE_FETCH_LIMIT),
                        commitment: Some(CommitmentConfig {
                            commitment: cfg.commitment.unwrap_or(CommitmentLevel::Confirmed),
                        }),
                    },
                )
                .await?;

            if sigs.is_empty() {
                break 'fetch_loop;
            }

            before_sig = sigs.last().and_then(|s| s.signature.parse().ok());

            // ищем последнюю известную сигнатуру
            if let Some(ref last_sig) = last_known_sig_str {
                if let Some(pos) = sigs.iter().position(|s| &s.signature == last_sig) {
                    signatures_to_process.extend_from_slice(&sigs[..pos]);
                    break 'fetch_loop;
                }
            }

            signatures_to_process.extend(sigs);
            fetched_count += signatures_to_process.len();

            // лимит по количеству сигнатур
            if let Some(max_sig_count) = cfg.max_signature_fetch {
                if fetched_count >= max_sig_count {
                    break 'fetch_loop;
                }
            }
        }

        // от старых к новым
        signatures_to_process.reverse();

        let current_slot = if cfg.max_catchup_depth.is_some() {
            Some(client.get_slot().await?)
        } else {
            None
        };

        for sig_info in signatures_to_process {
            // проверка по глубине слота
            if let (Some(max_depth), Some(curr_slot)) = (cfg.max_catchup_depth, current_slot) {
                if sig_info.slot < curr_slot.saturating_sub(max_depth) {
                    tracing::debug!(
                        "Skipping {} from slot {} due to max_catchup_depth",
                        sig_info.signature,
                        sig_info.slot
                    );
                    continue;
                }
            }

            let sig = sig_info.signature.parse::<Signature>()?;
            match client
                .get_transaction_with_config(
                    &sig,
                    RpcTransactionConfig {
                        encoding: Some(UiTransactionEncoding::Base64),
                        commitment: Some(CommitmentConfig {
                            commitment: cfg.commitment.unwrap_or(CommitmentLevel::Confirmed),
                        }),
                        max_supported_transaction_version: Some(0),
                    },
                )
                .await
            {
                Ok(tx) => {
                    if let Some(block_time) = tx.block_time {
                        let age_seconds = cfg.time_provider.timestamp().saturating_sub(block_time);
                        let max_age_seconds = cfg.max_request_age_minutes as i64 * 60;
                        if age_seconds > max_age_seconds {
                            tracing::debug!("Skipping {} due to age", sig);
                            storage.set_last_slot(tx.slot);
                            storage.set_last_sig(&sig.to_string());
                            continue;
                        }
                    }

                    if let Some(meta) = tx.transaction.meta {
                        if let Some(logs) = Option::<Vec<String>>::from(meta.log_messages) {
                            for l in logs {
                                if let Ok(ev) = try_parse_log(&l) {
                                    if !matches!(ev, BridgeEvent::Unknown) {
                                        if sender.send(ev).await.is_err() {
                                            return Err(anyhow::anyhow!("MPSC channel closed"));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    storage.set_last_slot(tx.slot);
                    storage.set_last_sig(&sig.to_string());
                }
                Err(e) => tracing::error!("Failed to get transaction {}: {}", sig, e),
            }
        }

        sleep(Duration::from_secs(cfg.poll_interval_secs.unwrap_or(3))).await;
    }
}
