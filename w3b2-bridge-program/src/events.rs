use super::*;

#[derive(Debug)]
#[event]
pub struct FundingRequested {
    pub user_wallet: Pubkey,
    pub target_admin: Pubkey,
    pub amount: u64,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct FundingApproved {
    pub user_wallet: Pubkey,
    pub approved_by: Pubkey,
    pub amount: u64,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct UserRegistered {
    pub owner: [u8; 32],
    pub account_type: WalletType,
    pub linked_wallet: Option<Pubkey>,
    pub ts: i64,
}

#[derive(Debug)]
#[event]
pub struct CommandEvent {
    pub sender: Pubkey,
    pub target_admin: Pubkey,
    pub command_id: u64,
    pub mode: CommandMode,
    pub payload: Vec<u8>,
    pub ts: i64,
}
