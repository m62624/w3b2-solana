use crate::config::SyncConfig;
use crate::storage::Storage;
use crate::{events::BridgeEvent, events::try_parse_log};

use anyhow::Result;
use chrono::Utc;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

const SIGNATURE_FETCH_LIMIT: usize = 1000;

/// catch-up worker: догоняет историю и отправляет события в канал
pub async fn run_catchup(
    cfg: SyncConfig,
    storage: Storage,
    sender: mpsc::Sender<BridgeEvent>,
) -> Result<()> {
    let client = RpcClient::new(cfg.rpc_url.clone());
    let program_id: solana_sdk::pubkey::Pubkey = cfg.program_id.parse()?;

    // Внешний цикл для периодической проверки больших разрывов в синхронизации
    loop {
        let mut before_sig: Option<Signature> = None;
        let last_known_sig_str = storage.get_last_sig();
        let mut signatures_to_process: Vec<RpcConfirmedTransactionStatusWithSignature> = Vec::new();

        tracing::info!(
            "Starting catch-up check from last known signature: {:?}",
            last_known_sig_str
        );

        // Внутренний цикл для постраничной загрузки всех новых транзакций
        'fetch_loop: loop {
            let sigs = client
                .get_signatures_for_address_with_config(
                    &program_id,
                    GetConfirmedSignaturesForAddress2Config {
                        before: before_sig,
                        until: None, // `until` не используем, проверяем вручную
                        limit: Some(SIGNATURE_FETCH_LIMIT),
                        commitment: Some(CommitmentConfig::confirmed()),
                    },
                )
                .await?;

            if sigs.is_empty() {
                break 'fetch_loop;
            }

            // Запоминаем последнюю (самую старую) подпись в пакете для следующего запроса
            before_sig = sigs.last().and_then(|s| s.signature.parse().ok());

            // Если мы уже что-то сохраняли, ищем эту подпись в текущем пакете
            if let Some(ref last_sig) = last_known_sig_str {
                if let Some(pos) = sigs.iter().position(|s| &s.signature == last_sig) {
                    // Нашли! Добавляем в обработку только те, что новее
                    signatures_to_process.extend_from_slice(&sigs[..pos]);
                    break 'fetch_loop;
                }
            }

            // Последняя обработанная подпись еще не найдена, добавляем весь пакет и идем глубже в историю
            signatures_to_process.extend(sigs);
        }

        // API возвращает транзакции от новых к старым. Разворачиваем, чтобы обрабатывать в хронологическом порядке.
        signatures_to_process.reverse();

        if !signatures_to_process.is_empty() {
            tracing::info!(
                "Found {} new transactions to process during catch-up.",
                signatures_to_process.len()
            );
        }

        // Оптимизация: получаем текущий слот один раз за цикл, если нужно
        let current_slot_for_depth_check = if cfg.max_catchup_depth.is_some() {
            Some(client.get_slot().await?)
        } else {
            None
        };

        for sig_info in signatures_to_process {
            // Применяем ограничение по глубине синхронизации, если оно задано
            if let (Some(max_depth), Some(current_slot)) =
                (cfg.max_catchup_depth, current_slot_for_depth_check)
            {
                if sig_info.slot < current_slot.saturating_sub(max_depth) {
                    tracing::debug!(
                        "Skipping signature {} from slot {} due to max_catchup_depth",
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
                        encoding: Some(solana_transaction_status::UiTransactionEncoding::Base64),
                        commitment: Some(CommitmentConfig::confirmed()),
                        max_supported_transaction_version: Some(0),
                    },
                )
                .await
            {
                Ok(tx) => {
                    // Проверяем возраст транзакции, если он доступен
                    if let Some(block_time) = tx.block_time {
                        let now = Utc::now().timestamp();
                        let age_seconds = now.saturating_sub(block_time);
                        let max_age_seconds = cfg.max_request_age_minutes as i64 * 60;

                        if age_seconds > max_age_seconds {
                            tracing::debug!(
                                "Skipping transaction {} due to age ({}s old > max {}s).",
                                sig,
                                age_seconds,
                                max_age_seconds
                            );
                            // Важно обновить состояние, чтобы не проверять эту старую транзакцию снова
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
                                            return Err(anyhow::anyhow!("MPSC channel closed."));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Обновляем состояние в БД *после* успешной обработки и отправки событий
                    storage.set_last_slot(tx.slot);
                    storage.set_last_sig(&sig.to_string());
                }
                Err(e) => tracing::error!("Failed to get transaction {}: {}", sig, e),
            }
        }

        // Вне зависимости от того, были ли новые транзакции, делаем паузу перед следующей проверкой.
        tracing::info!("Catch-up cycle finished. Pausing for 60 seconds before next check.");
        sleep(Duration::from_secs(60)).await;
    }
}
