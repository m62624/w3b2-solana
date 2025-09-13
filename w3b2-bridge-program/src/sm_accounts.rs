use super::*;

#[account]
#[derive(Debug)]
pub struct AccountMeta {
    pub owner: Pubkey,
    pub co_signer: Pubkey,
    pub active: bool,
}

impl AccountMeta {
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

#[account]
#[derive(Debug)]
pub struct AdminAccount {
    pub meta: AccountMeta,
}

#[account]
#[derive(Debug)]
pub struct UserAccount {
    pub meta: AccountMeta,
}

#[derive(Debug, Accounts)]
pub struct RegisterAdmin<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 1, // discriminator + owner + co_signer + active
        seeds = [b"admin", co_signer.key().as_ref()],
        bump
    )]
    pub admin_account: Account<'info, AdminAccount>,

    #[account(mut)]
    pub payer: Signer<'info>, // основной кошелек
    pub authority: Signer<'info>, // основной кошелек (тот же)
    pub co_signer: Signer<'info>, // уникальный ключ для этого PDA
    pub system_program: Program<'info, System>,
}

#[derive(Debug, Accounts)]
pub struct RegisterUser<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 8, // discriminator + owner + co_signer + balance
        seeds = [b"user", co_signer.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(mut)]
    pub payer: Signer<'info>, // основной кошелек
    pub user_wallet: Signer<'info>, // владелец пользователя
    pub co_signer: Signer<'info>,   // уникальный co-signer для PDA
    pub system_program: Program<'info, System>,
}

#[derive(Debug, Accounts)]
pub struct DeactivateAdmin<'info> {
    #[account(mut)]
    pub admin_account: Account<'info, AdminAccount>,
}

#[derive(Debug, Accounts)]
pub struct DeactivateUser<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}

/// Funding request storage
#[account]
#[derive(Debug)]
pub struct FundingRequest {
    pub user_wallet: Pubkey,
    pub target_admin: Pubkey,
    pub amount: u64,
    pub status: u8,
}

#[derive(Debug, Accounts)]
pub struct RequestFunding<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 8 + 1,
        seeds = [b"funding", user_account.key().as_ref(), &payer.key().to_bytes()],
        bump
    )]
    pub funding_request: Account<'info, FundingRequest>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub user_account: Account<'info, UserAccount>,
    pub system_program: Program<'info, System>,
}

/// Admin approves funding request
#[derive(Debug, Accounts)]
pub struct ApproveFunding<'info> {
    #[account(
        mut,
        seeds = [b"admin", admin_authority.key().as_ref()],
        bump,
    )]
    pub admin_account: Account<'info, AdminAccount>,
    #[account(mut)]
    pub funding_request: Account<'info, FundingRequest>,
    #[account(mut)]
    pub user_wallet: SystemAccount<'info>,
    pub admin_authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Debug, Accounts)]
pub struct DispatchCommandAdmin<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub admin_account: Account<'info, AdminAccount>,
}

#[derive(Debug, Accounts)]
pub struct DispatchCommandUser<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}
