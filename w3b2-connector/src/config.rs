use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// RPC HTTP endpoint
    pub rpc_url: String,
    /// RPC WS endpoint
    pub ws_url: String,
    /// Address of your program
    pub program_id: String,
    /// Max slots to go back during catch-up
    pub max_catchup_depth: u64,
    /// Max age of a funding request in minutes to be processed
    pub max_request_age_minutes: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8899".into(),
            ws_url: "ws://127.0.0.1:8900".into(),
            program_id: w3b2_bridge_program::ID.to_string(),
            max_catchup_depth: 5_000,
            max_request_age_minutes: 60, // 1 час
        }
    }
}
