use anyhow::Result;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::BorshDeserialize;
use w3b2_bridge_program::events::{
    AdminDeactivated, AdminRegistered, FundingRequested, UserDeactivated, UserRegistered,
};
use w3b2_bridge_program::events::{CommandEvent, FundingApproved};

#[derive(Debug)]
pub enum BridgeEvent {
    AdminRegistered(AdminRegistered),
    UserRegistered(UserRegistered),
    AdminDeactivated(AdminDeactivated),
    UserDeactivated(UserDeactivated),
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

    let admin_registered_disc =
        anchor_lang::solana_program::hash::hash(b"event:AdminRegistered").to_bytes()[0..8].to_vec();
    let user_registered_disc =
        anchor_lang::solana_program::hash::hash(b"event:UserRegistered").to_bytes()[0..8].to_vec();
    let admin_deactivated_disc = anchor_lang::solana_program::hash::hash(b"event:AdminDeactivated")
        .to_bytes()[0..8]
        .to_vec();
    let user_deactivated_disc =
        anchor_lang::solana_program::hash::hash(b"event:UserDeactivated").to_bytes()[0..8].to_vec();
    let funding_requested_disc = anchor_lang::solana_program::hash::hash(b"event:FundingRequested")
        .to_bytes()[0..8]
        .to_vec();
    let funding_approved_disc =
        anchor_lang::solana_program::hash::hash(b"event:FundingApproved").to_bytes()[0..8].to_vec();
    let command_event_disc =
        anchor_lang::solana_program::hash::hash(b"event:CommandEvent").to_bytes()[0..8].to_vec();

    if discriminator == admin_registered_disc.as_slice() {
        let event = AdminRegistered::try_from_slice(event_data)?;
        return Ok(BridgeEvent::AdminRegistered(event));
    }

    if discriminator == user_registered_disc.as_slice() {
        let event = UserRegistered::try_from_slice(event_data)?;
        return Ok(BridgeEvent::UserRegistered(event));
    }

    if discriminator == admin_deactivated_disc.as_slice() {
        let event = AdminDeactivated::try_from_slice(event_data)?;
        return Ok(BridgeEvent::AdminDeactivated(event));
    }

    if discriminator == user_deactivated_disc.as_slice() {
        let event = UserDeactivated::try_from_slice(event_data)?;
        return Ok(BridgeEvent::UserDeactivated(event));
    }

    if discriminator == funding_requested_disc.as_slice() {
        let event = FundingRequested::try_from_slice(event_data)?;
        return Ok(BridgeEvent::FundingRequested(event));
    }

    if discriminator == funding_approved_disc.as_slice() {
        let event = FundingApproved::try_from_slice(event_data)?;
        return Ok(BridgeEvent::FundingApproved(event));
    }

    if discriminator == command_event_disc.as_slice() {
        let event = CommandEvent::try_from_slice(event_data)?;
        return Ok(BridgeEvent::CommandEvent(event));
    }

    Ok(BridgeEvent::Unknown)
}

/// Попытка извлечь base64-пайс из строки логов и распарсить событие.
pub fn try_parse_log(log: &str) -> Result<BridgeEvent> {
    let candidates = if let Some(s) = log.strip_prefix("Program data: ") {
        vec![s]
    } else if let Some(s) = log.strip_prefix("Program log: ") {
        vec![s]
    } else {
        vec![log]
    };

    for cand in candidates {
        for raw_token in cand.split_whitespace() {
            let token =
                raw_token.trim_matches(|c: char| matches!(c, '"' | '\'' | '[' | ']' | ',' | ':'));
            if token.len() < 12 {
                continue;
            }

            if let Ok(bytes) = BASE64.decode(token) {
                if let Ok(ev) = parse_event_data(&bytes) {
                    if !matches!(ev, BridgeEvent::Unknown) {
                        return Ok(ev);
                    }
                }
            }
        }
    }

    Ok(BridgeEvent::Unknown)
}
