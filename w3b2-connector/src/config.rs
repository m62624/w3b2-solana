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
    /// Whether to skip old events (true = strict TTL)
    pub skip_stale: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8899".into(),
            ws_url: "ws://127.0.0.1:8900".into(),
            program_id: w3b2_bridge_program::ID.to_string(),
            max_catchup_depth: 10_000,
            skip_stale: false,
        }
    }
}
