use super::*;

#[derive(Debug)]
#[event]
pub struct FundingRequested {
    pub user_wallet: Pubkey,
    pub amount: u64,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct FundingApproved {
    pub user_wallet: Pubkey,
    pub amount: u64,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct UserRegistered {
    /// registrant pubkey as raw bytes ([u8;32])
    pub owner: [u8; 32],
    pub account_type: WalletType,
    pub linked_wallet: Option<[u8; 32]>,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct CommandEvent {
    /// sender pubkey as raw bytes ([u8;32])
    pub sender: [u8; 32],
    pub command_id: u64,
    pub mode: CommandMode,
    pub payload: Vec<u8>,
    pub ts: i64,
}
