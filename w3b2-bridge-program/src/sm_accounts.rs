use super::*;
use anchor_lang::prelude::*;

/// Admin registration
#[derive(Debug, Accounts)]
pub struct RegisterAdmin<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + 32, // discriminator + owner
        seeds = [b"admin", authority.key().as_ref()],
        bump
    )]
    pub admin_profile: Account<'info, AdminAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

/// User funding request
#[derive(Debug, Accounts)]
pub struct RequestFunding<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 8 + 1, // discriminator + user_wallet + amount + status
        seeds = [b"funding", user_wallet.key().as_ref(), &payer.key().to_bytes()],
        bump
    )]
    pub funding_request: Account<'info, FundingRequest>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub user_wallet: SystemAccount<'info>,
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
    pub admin_profile: Account<'info, AdminAccount>,

    #[account(mut)]
    pub funding_request: Account<'info, FundingRequest>,

    pub admin_authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

/// Admin executes transfer to user wallet
#[derive(Debug, Accounts)]
pub struct FundUserWallet<'info> {
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
    )]
    pub admin_profile: Account<'info, AdminAccount>,

    #[account(mut)]
    pub user_wallet: SystemAccount<'info>,

    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

/// Command dispatcher
#[derive(Debug, Accounts)]
pub struct DispatchCommand<'info> {
    #[account(mut, seeds = [b"user", authority.key().as_ref()], bump)]
    pub user_pda: Account<'info, UserPda>,
    pub authority: Signer<'info>,
}

/// Admin account storage
#[account]
#[derive(Debug)]
pub struct AdminAccount {
    pub owner: Pubkey,
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

/// User PDA
#[account]
#[derive(Debug)]
pub struct UserPda {
    pub profile: UserAccount,
    pub linked_wallet: Option<Pubkey>,
    pub created_at: u64,
}

/// User registration
#[derive(Debug, Accounts)]
pub struct RegisterUser<'info> {
    #[account(
        init_if_needed,
        payer = payer,
        space = 96,
        seeds = [b"user", authority.key().as_ref()],
        bump
    )]
    pub user_pda: Account<'info, UserPda>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}
