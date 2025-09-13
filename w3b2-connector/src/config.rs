use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::commitment_config::CommitmentLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// RPC HTTP endpoint
    pub rpc_url: String,
    /// RPC WS endpoint
    pub ws_url: String,
    /// Address of your program
    pub program_id: String,
    /// Max slots to go back during catch-up. `None` means unlimited.
    pub max_catchup_depth: Option<u64>,
    /// Max age of a funding request in minutes to be processed
    pub max_request_age_minutes: u64,
    pub time_provider: DateTime<Utc>,
    /// Poll interval in seconds for catch-up worker
    /// Default: 3 seconds
    pub poll_interval_secs: Option<u64>,
    /// Commitment level for RPC requests
    /// Default: Confirmed
    pub commitment: Option<solana_sdk::commitment_config::CommitmentLevel>,
    /// Max number of signatures to fetch in one RPC call
    pub max_signature_fetch: Option<usize>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8899".into(),
            ws_url: "ws://127.0.0.1:8900".into(),
            program_id: w3b2_bridge_program::ID.to_string(),
            max_catchup_depth: None,
            max_request_age_minutes: 60, // 1 час
            time_provider: Utc::now(),
            poll_interval_secs: Some(3),
            commitment: Some(CommitmentLevel::Confirmed),
            max_signature_fetch: None,
        }
    }
}
