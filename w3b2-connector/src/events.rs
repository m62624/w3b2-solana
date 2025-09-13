use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use borsh::BorshDeserialize;
use w3b2_bridge_program::events::FundingRequested;
use w3b2_bridge_program::events::{CommandEvent, FundingApproved};

#[derive(Debug)]
pub enum BridgeEvent {
    FundingRequested(FundingRequested),
    FundingApproved(FundingApproved),
    CommandEvent(CommandEvent),
    Unknown,
}

pub fn parse_event_data(data: &[u8]) -> Result<BridgeEvent> {
    if data.len() < 8 {
        return Ok(BridgeEvent::Unknown);
    }

    let discriminator = &data[0..8];
    let event_data = &data[8..];

    let funding_requested_disc =
        anchor_lang::solana_program::hash::hash(b"global:funding_requested").to_bytes()[0..8]
            .to_vec();
    let funding_approved_disc = anchor_lang::solana_program::hash::hash(b"global:funding_approved")
        .to_bytes()[0..8]
        .to_vec();
    let command_event_disc =
        anchor_lang::solana_program::hash::hash(b"global:command_event").to_bytes()[0..8].to_vec();

    match discriminator {
        _ if discriminator == funding_requested_disc.as_slice() => {
            let event = FundingRequested::try_from_slice(event_data)?;
            Ok(BridgeEvent::FundingRequested(event))
        }
        _ if discriminator == funding_approved_disc.as_slice() => {
            let event = FundingApproved::try_from_slice(event_data)?;
            Ok(BridgeEvent::FundingApproved(event))
        }
        _ if discriminator == command_event_disc.as_slice() => {
            let event = CommandEvent::try_from_slice(event_data)?;
            Ok(BridgeEvent::CommandEvent(event))
        }
        _ => Ok(BridgeEvent::Unknown),
    }
}

pub fn try_parse_log(log: &str) -> Result<BridgeEvent> {
    if let Some(stripped) = log.strip_prefix("Program log: ") {
        if let Ok(data) = BASE64.decode(stripped) {
            return parse_event_data(&data);
        }
    }
    Ok(BridgeEvent::Unknown)
}
