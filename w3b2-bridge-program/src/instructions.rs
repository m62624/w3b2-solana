use super::*;
use solana_program::program::invoke;
use solana_program::system_instruction;

fn register_pda<'info, T: AccountSerialize + AccountDeserialize + Clone>(
    pda_account: &mut Account<'info, T>,
    payer: &Signer<'info>,
    system_program: &Program<'info, System>,
    lamports: u64,
) -> Result<()> {
    let ix =
        system_instruction::transfer(&payer.key(), &pda_account.to_account_info().key, lamports);
    invoke(
        &ix,
        &[
            payer.to_account_info(),
            pda_account.to_account_info(),
            system_program.to_account_info(),
        ],
    )?;
    Ok(())
}

pub fn register_admin(ctx: Context<RegisterAdmin>, initial_balance: u64) -> Result<()> {
    let admin = &mut ctx.accounts.admin_account;
    admin.meta.owner = ctx.accounts.authority.key();
    admin.meta.co_signer = ctx.accounts.co_signer.key();
    admin.meta.active = true;

    require!(
        ctx.accounts.payer.lamports() >= initial_balance,
        BridgeError::InsufficientFundsForAdmin
    );

    register_pda(
        admin,
        &ctx.accounts.payer,
        &ctx.accounts.system_program,
        initial_balance,
    )?;

    emit!(AdminRegistered {
        admin: ctx.accounts.authority.key(),
        initial_funding: initial_balance,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn register_user(ctx: Context<RegisterUser>, initial_balance: u64) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    user.meta.owner = ctx.accounts.user_wallet.key();
    user.meta.co_signer = ctx.accounts.co_signer.key();
    user.meta.active = true;

    register_pda(
        user,
        &ctx.accounts.payer,
        &ctx.accounts.system_program,
        initial_balance,
    )?;

    emit!(UserRegistered {
        user: ctx.accounts.user_wallet.key(),
        initial_balance,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn deactivate_admin(ctx: Context<DeactivateAdmin>) -> Result<()> {
    let admin = &mut ctx.accounts.admin_account;
    admin.meta.deactivate();

    emit!(AdminDeactivated {
        admin: admin.meta.owner,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn deactivate_user(ctx: Context<DeactivateUser>) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    user.meta.deactivate();

    emit!(UserDeactivated {
        user: user.meta.owner,
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
    funding_request.user_wallet = ctx.accounts.user_account.meta.owner;
    funding_request.amount = amount;
    funding_request.status = FundingStatus::Pending as u8;
    funding_request.target_admin = target_admin;

    emit!(FundingRequested {
        user_wallet: funding_request.user_wallet,
        amount,
        target_admin,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn approve_funding(ctx: Context<ApproveFunding>) -> Result<()> {
    let funding_request = &mut ctx.accounts.funding_request;

    let admin_account = &mut ctx.accounts.admin_account;

    require!(admin_account.meta.active, BridgeError::InactiveAccount);
    require!(
        funding_request.target_admin == ctx.accounts.admin_authority.key(),
        BridgeError::Unauthorized
    );
    require!(
        funding_request.status == FundingStatus::Pending as u8,
        BridgeError::RequestAlreadyProcessed
    );
    require!(
        admin_account.to_account_info().lamports() >= funding_request.amount,
        BridgeError::InsufficientFundsForFunding
    );

    **admin_account.to_account_info().try_borrow_mut_lamports()? -= funding_request.amount;
    **ctx
        .accounts
        .user_wallet
        .to_account_info()
        .try_borrow_mut_lamports()? += funding_request.amount;

    funding_request.status = FundingStatus::Approved as u8;

    emit!(FundingApproved {
        user_wallet: funding_request.user_wallet,
        amount: funding_request.amount,
        approved_by: funding_request.target_admin,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn dispatch_command_admin(
    ctx: Context<DispatchCommandAdmin>,
    command_id: u64,
    mode: CommandMode,
    payload: Vec<u8>,
    target_pubkey: Pubkey,
) -> Result<()> {
    require!(payload.len() <= 1024, BridgeError::PayloadTooLarge);

    let admin = &ctx.accounts.admin_account;
    require!(admin.meta.active, BridgeError::InactiveAccount);
    require!(admin.meta.owner == target_pubkey, BridgeError::Unauthorized);

    emit!(CommandEvent {
        sender: ctx.accounts.authority.key(),
        target: target_pubkey,
        command_id,
        mode,
        payload,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn dispatch_command_user(
    ctx: Context<DispatchCommandUser>,
    command_id: u64,
    mode: CommandMode,
    payload: Vec<u8>,
    target_pubkey: Pubkey,
) -> Result<()> {
    require!(payload.len() <= 1024, BridgeError::PayloadTooLarge);

    let user = &ctx.accounts.user_account;
    require!(user.meta.active, BridgeError::InactiveAccount);
    require!(user.meta.owner == target_pubkey, BridgeError::Unauthorized);

    emit!(CommandEvent {
        sender: ctx.accounts.authority.key(),
        target: target_pubkey,
        command_id,
        mode,
        payload,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}
