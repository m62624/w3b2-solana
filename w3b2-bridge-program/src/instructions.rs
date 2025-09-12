use super::*;
use solana_program::{program::invoke_signed, system_instruction};

pub fn register_admin(ctx: Context<RegisterAdmin>) -> Result<()> {
    let admin_profile = &mut ctx.accounts.admin_profile;
    admin_profile.owner = ctx.accounts.authority.key().to_bytes();
    Ok(())
}

pub fn request_funding(ctx: Context<RequestFunding>, amount: u64) -> Result<()> {
    let funding_request = &mut ctx.accounts.funding_request;

    // Set the request details
    funding_request.user_wallet = ctx.accounts.user_wallet.key();
    funding_request.amount = amount;
    funding_request.status = FundingStatus::Pending as u8;

    // Emit event for off-chain service to pick up
    emit!(FundingRequested {
        user_wallet: funding_request.user_wallet,
        amount: funding_request.amount,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn approve_funding(ctx: Context<ApproveFunding>) -> Result<()> {
    let admin_profile = &ctx.accounts.admin_profile;
    let funding_request = &mut ctx.accounts.funding_request;

    // Check that the admin is the one approving
    require!(
        admin_profile.owner == ctx.accounts.admin_authority.key().to_bytes(),
        BridgeError::Unauthorized
    );

    // Check if the request is still pending
    require!(
        funding_request.status == FundingStatus::Pending as u8,
        BridgeError::RequestAlreadyProcessed
    );

    // Get the bump seed for the PDA.
    let bump = ctx.bumps.admin_profile;

    // Define the seeds for the PDA signer
    let pda_seeds = &[b"admin".as_ref(), admin_profile.owner.as_ref(), &[bump]];

    // Create the transfer instruction
    let transfer_instruction = system_instruction::transfer(
        ctx.accounts.admin_profile.to_account_info().key,
        &funding_request.user_wallet,
        funding_request.amount,
    );

    let funding_request_info = ctx.accounts.funding_request.to_account_info();
    let funding_request = &mut ctx.accounts.funding_request;

    invoke_signed(
        &transfer_instruction,
        &[
            ctx.accounts.admin_profile.to_account_info(),
            funding_request_info,
            ctx.accounts.admin_authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[pda_seeds],
    )?;

    // Update the request status
    funding_request.status = FundingStatus::Approved as u8;

    // Emit event for off-chain listeners
    emit!(FundingApproved {
        user_wallet: funding_request.user_wallet,
        amount: funding_request.amount,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn dispatch_command(
    ctx: Context<DispatchCommand>,
    command_id: u64,
    mode: CommandMode,
    payload: Vec<u8>,
) -> Result<()> {
    require!(payload.len() <= 1024, BridgeError::PayloadTooLarge);

    let pda = &ctx.accounts.user_pda;
    let signer_bytes = ctx.accounts.authority.key().to_bytes();

    // signer must be owner or linked_wallet
    let is_owner = pda.profile.owner == signer_bytes;
    let is_linked = pda.linked_wallet.map_or(false, |lk| lk == signer_bytes);
    require!(is_owner || is_linked, BridgeError::Unauthorized);

    let ts = clock::Clock::get()?.unix_timestamp;

    emit!(CommandEvent {
        sender: signer_bytes,
        command_id,
        mode,
        payload,
        ts,
    });

    Ok(())
}
