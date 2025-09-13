use crate::config::SyncConfig;
use crate::storage::Storage;
use crate::{events::BridgeEvent, events::try_parse_log};

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use tokio::time::{Duration, sleep};

/// catch-up worker: догоняет историю и возвращает события
pub async fn run_catchup(cfg: SyncConfig, storage: Storage) -> Result<Vec<BridgeEvent>> {
    let client = RpcClient::new(cfg.rpc_url.clone());
    let mut collected = Vec::new();

    loop {
        let last_slot = storage.get_last_slot();
        let current_slot = client.get_slot().await?;
        if current_slot <= last_slot {
            sleep(Duration::from_secs(3)).await;
            return Ok(collected); // пока нечего нового
        }

        let sigs = client
            .get_signatures_for_address_with_config(
                &cfg.program_id.parse()?,
                GetConfirmedSignaturesForAddress2Config {
                    before: None,
                    until: None,
                    limit: Some(1000),
                    commitment: Some(CommitmentConfig::confirmed()),
                },
            )
            .await?;

        for sig_info in sigs {
            let sig = sig_info.signature.parse::<Signature>()?;
            if let Ok(tx) = client
                .get_transaction_with_config(
                    &sig,
                    RpcTransactionConfig {
                        encoding: Some(solana_transaction_status::UiTransactionEncoding::Base64),
                        commitment: None,
                        max_supported_transaction_version: Some(0),
                    },
                )
                .await
            {
                if let Some(meta) = tx.transaction.meta {
                    if let Some(logs) = Option::<Vec<String>>::from(meta.log_messages) {
                        for l in logs {
                            if l.contains(&cfg.program_id) {
                                if let Ok(ev) = try_parse_log(&l) {
                                    collected.push(ev);
                                }
                            }
                        }
                    }
                }
            }
        }

        storage.set_last_slot(current_slot);
        return Ok(collected);
    }
}
