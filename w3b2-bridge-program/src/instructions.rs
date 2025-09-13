use super::*;
use solana_program::program::invoke;
use solana_program::system_instruction;

fn check_rent_and_balance(account: &AccountInfo, additional: u64) -> Result<bool> {
    let rent = Rent::get()?;
    let min_balance = rent.minimum_balance(account.data_len());
    Ok(account.lamports() >= min_balance.saturating_add(additional))
}

pub fn register_admin(ctx: Context<RegisterAdmin>, funding_amount: u64) -> Result<()> {
    {
        let admin_profile = &mut ctx.accounts.admin_profile;
        admin_profile.owner = ctx.accounts.authority.key();

        require!(
            ctx.accounts.payer.lamports() >= funding_amount,
            BridgeError::InsufficientFundsForAdmin
        );
    }

    let ix = system_instruction::transfer(
        &ctx.accounts.payer.key(),
        &ctx.accounts.admin_profile.key(),
        funding_amount,
    );

    invoke(
        &ix,
        &[
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.admin_profile.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    emit!(AdminRegistered {
        admin: ctx.accounts.authority.key(),
        initial_funding: funding_amount,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}
pub fn request_funding(
    ctx: Context<RequestFunding>,
    amount: u64,
    target_admin: Pubkey,
) -> Result<()> {
    let funding_request = &mut ctx.accounts.funding_request;

    funding_request.user_wallet = ctx.accounts.user_wallet.key();
    funding_request.amount = amount;
    funding_request.status = FundingStatus::Pending as u8;
    funding_request.target_admin = target_admin;

    emit!(FundingRequested {
        user_wallet: funding_request.user_wallet,
        amount: funding_request.amount,
        target_admin: funding_request.target_admin,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn approve_funding(ctx: Context<ApproveFunding>) -> Result<()> {
    let funding_request = &mut ctx.accounts.funding_request;

    //Checks the admin to whom the request was originally sent (target_admin in FundingRequest).
    // Each request is linked to a specific administrator, and only he can approve or reject the request.
    require!(
        funding_request.target_admin == ctx.accounts.admin_authority.key(),
        BridgeError::Unauthorized
    );

    require!(
        funding_request.status == FundingStatus::Pending as u8,
        BridgeError::RequestAlreadyProcessed
    );

    require!(
        check_rent_and_balance(
            &ctx.accounts.admin_profile.to_account_info(),
            funding_request.amount
        )?,
        BridgeError::InsufficientFundsForFunding
    );

    // decrease the admin profile balance
    ctx.accounts
        .admin_profile
        .to_account_info()
        .sub_lamports(funding_request.amount)?;

    // increase the user wallet balance
    ctx.accounts
        .user_wallet
        .to_account_info()
        .add_lamports(funding_request.amount)?;

    // Update the request status
    funding_request.status = FundingStatus::Approved as u8;

    // Emit event for off-chain listeners
    emit!(FundingApproved {
        user_wallet: funding_request.user_wallet,
        amount: funding_request.amount,
        approved_by: funding_request.target_admin,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn dispatch_command(
    ctx: Context<DispatchCommand>,
    command_id: u64,
    mode: CommandMode,
    payload: Vec<u8>,
    target_admin: Pubkey,
) -> Result<()> {
    require!(payload.len() <= 1024, BridgeError::PayloadTooLarge);

    let signer = ctx.accounts.authority.key(); // user_wallet

    emit!(CommandEvent {
        sender: signer,
        target_admin,
        command_id,
        mode,
        payload,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}
