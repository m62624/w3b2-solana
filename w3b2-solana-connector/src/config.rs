#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use solana_sdk::commitment_config::CommitmentLevel;

/// The top-level configuration for the `w3b2-solana-connector` library.
///
/// This struct aggregates all necessary settings, including Solana network endpoints
/// and synchronizer behavior. It is typically deserialized from a configuration file
/// and passed to the `EventManager` upon initialization.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub struct ConnectorConfig {
    #[cfg_attr(feature = "serde", serde(default))]
    pub solana: Solana,
    #[cfg_attr(feature = "serde", serde(default))]
    pub synchronizer: Synchronizer,
    #[cfg_attr(feature = "serde", serde(default))]
    pub channels: ChannelConfig,
}

/// Defines the connection settings for the Solana cluster.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub struct Solana {
    pub rpc_url: String,
    pub ws_url: String,
    #[cfg_attr(feature = "serde", serde(with = "serde_commitment"))]
    pub commitment: CommitmentLevel,
}

/// Defines behavior for the event synchronization workers (`LiveWorker` and `CatchupWorker`).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub struct Synchronizer {
    /// The maximum number of slots to look back during catch-up. `None` means no limit.
    pub max_catchup_depth: Option<u64>,
    /// The interval in seconds at which the `CatchupWorker` polls for historical transactions.
    pub poll_interval_secs: u64,
    /// The maximum number of signatures to fetch in a single RPC call during catch-up.
    pub max_signature_fetch: usize,
}

/// Defines capacities for various MPSC channels within the connector.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub struct ChannelConfig {
    /// The buffer capacity for the dispatcher's internal event queue.
    pub dispatcher_event_buffer: usize,
    /// The buffer capacity for the command channel to the Dispatcher.
    pub dispatcher_command_buffer: usize,
    /// The default buffer capacity for individual listener channels (e.g., UserListener).
    pub listener_event_buffer: usize,
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            solana: Solana::default(),
            synchronizer: Synchronizer::default(),
            channels: ChannelConfig::default(),
        }
    }
}

impl Default for Solana {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8899".to_string(),
            ws_url: "ws://127.0.0.1:8900".to_string(),
            commitment: CommitmentLevel::Confirmed,
        }
    }
}

impl Default for Synchronizer {
    fn default() -> Self {
        Self {
            max_catchup_depth: None,
            poll_interval_secs: 3,
            max_signature_fetch: 1000,
        }
    }
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            dispatcher_event_buffer: 256,
            dispatcher_command_buffer: 128,
            listener_event_buffer: 128,
        }
    }
}

#[cfg(feature = "serde")]
mod serde_commitment {

    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(c: &CommitmentLevel, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match c {
            CommitmentLevel::Processed => "Processed",
            CommitmentLevel::Confirmed => "Confirmed",
            CommitmentLevel::Finalized => "Finalized",
        };
        serializer.serialize_str(s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<CommitmentLevel, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let level = match s.to_lowercase().as_str() {
            "processed" => CommitmentLevel::Processed,
            "confirmed" => CommitmentLevel::Confirmed,
            "finalized" => CommitmentLevel::Finalized,
            _ => CommitmentLevel::Confirmed,
        };
        Ok(level)
    }
}
