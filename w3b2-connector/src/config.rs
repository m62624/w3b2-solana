use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// RPC HTTP endpoint
    pub rpc_url: String,
    /// RPC WS endpoint
    pub ws_url: String,
    /// Address of your program
    pub program_id: String,
    /// Max slots to go back during catch-up. `None` means unlimited.
    #[serde(default = "default_max_catchup_depth")]
    pub max_catchup_depth: Option<u64>,
    /// Max age of a funding request in minutes to be processed
    pub max_request_age_minutes: u64,
}

/// Helper for serde to default to `None` for `max_catchup_depth`.
fn default_max_catchup_depth() -> Option<u64> {
    None
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8899".into(),
            ws_url: "ws://127.0.0.1:8900".into(),
            program_id: w3b2_bridge_program::ID.to_string(),
            max_catchup_depth: default_max_catchup_depth(),
            max_request_age_minutes: 60, // 1 час
        }
    }
}
