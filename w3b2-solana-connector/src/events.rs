use anchor_lang::AnchorDeserialize;
use anchor_lang::Discriminator;
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use w3b2_solana_program::events as OnChainEvent;

/// Indicates the origin of a `BridgeEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSource {
    /// The event was fetched by the `LiveWorker` from a real-time WebSocket stream.
    Live,
    /// The event was fetched by the `CatchupWorker` from historical transactions.
    Catchup,
}

/// A connector-side enum that wraps all possible on-chain events.
/// This provides a single, unified type for the dispatcher to work with.
#[derive(Debug, Clone)]
pub struct BridgeEvent {
    pub source: EventSource,
    pub data: BridgeEventData,
}
#[derive(Debug, Clone)]
pub enum BridgeEventData {
    AdminProfileRegistered(OnChainEvent::AdminProfileRegistered),
    AdminConfigUpdated(OnChainEvent::AdminConfigUpdated),
    AdminFundsWithdrawn(OnChainEvent::AdminFundsWithdrawn),
    AdminProfileClosed(OnChainEvent::AdminProfileClosed),
    AdminCommandDispatched(OnChainEvent::AdminCommandDispatched),
    UserProfileCreated(OnChainEvent::UserProfileCreated),
    UserCommKeyUpdated(OnChainEvent::UserCommKeyUpdated),
    UserFundsDeposited(OnChainEvent::UserFundsDeposited),
    UserFundsWithdrawn(OnChainEvent::UserFundsWithdrawn),
    UserProfileClosed(OnChainEvent::UserProfileClosed),
    UserCommandDispatched(OnChainEvent::UserCommandDispatched),
    OffChainActionLogged(OnChainEvent::OffChainActionLogged),
    Unknown,
}

pub fn try_parse_log(log: &str) -> Result<BridgeEvent> {
    if let Some(data_str) = log.strip_prefix("Program data: ") {
        if let Ok(bytes) = BASE64.decode(data_str.trim()) {
            let data = &bytes;

            fn try_match<E, F>(data: &[u8], map: F) -> Option<BridgeEventData>
            where
                E: AnchorDeserialize + Discriminator,
                F: FnOnce(E) -> BridgeEventData,
            {
                let disc = E::DISCRIMINATOR;
                if data.starts_with(&disc) {
                    if let Ok(e) = E::try_from_slice(&data[disc.len()..]) {
                        return Some(map(e));
                    }
                }
                None
            }

            let event_data = try_match::<OnChainEvent::AdminProfileRegistered, _>(
                data,
                BridgeEventData::AdminProfileRegistered,
            )
            .or_else(|| {
                try_match::<OnChainEvent::AdminConfigUpdated, _>(
                    data,
                    BridgeEventData::AdminConfigUpdated,
                )
            })
            .or_else(|| {
                try_match::<OnChainEvent::AdminFundsWithdrawn, _>(
                    data,
                    BridgeEventData::AdminFundsWithdrawn,
                )
            })
            .or_else(|| {
                try_match::<OnChainEvent::AdminProfileClosed, _>(
                    data,
                    BridgeEventData::AdminProfileClosed,
                )
            })
            .or_else(|| {
                try_match::<OnChainEvent::AdminCommandDispatched, _>(
                    data,
                    BridgeEventData::AdminCommandDispatched,
                )
            })
            .or_else(|| {
                try_match::<OnChainEvent::UserProfileCreated, _>(
                    data,
                    BridgeEventData::UserProfileCreated,
                )
            })
            .or_else(|| {
                try_match::<OnChainEvent::UserCommKeyUpdated, _>(
                    data,
                    BridgeEventData::UserCommKeyUpdated,
                )
            })
            .or_else(|| {
                try_match::<OnChainEvent::UserFundsDeposited, _>(
                    data,
                    BridgeEventData::UserFundsDeposited,
                )
            })
            .or_else(|| {
                try_match::<OnChainEvent::UserFundsWithdrawn, _>(
                    data,
                    BridgeEventData::UserFundsWithdrawn,
                )
            })
            .or_else(|| {
                try_match::<OnChainEvent::UserProfileClosed, _>(
                    data,
                    BridgeEventData::UserProfileClosed,
                )
            })
            .or_else(|| {
                try_match::<OnChainEvent::UserCommandDispatched, _>(
                    data,
                    BridgeEventData::UserCommandDispatched,
                )
            })
            .or_else(|| {
                try_match::<OnChainEvent::OffChainActionLogged, _>(
                    data,
                    BridgeEventData::OffChainActionLogged,
                )
            })
            .unwrap_or(BridgeEventData::Unknown);

            if !matches!(event_data, BridgeEventData::Unknown) {
                return Ok(BridgeEvent {
                    source: EventSource::Catchup,
                    data: event_data,
                });
            }
        }
    }

    Err(anyhow::anyhow!("Log is not a valid program event"))
}
