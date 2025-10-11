#![allow(dead_code)]

use super::*;

pub fn create_profile(
    svm: &mut LiteSVM,
    authority: &Keypair,
    comm_key: Pubkey,
    target_admin_pda: Pubkey,
) -> Pubkey {
    let (create_ix, user_pda) = ix_create_profile(authority, comm_key, target_admin_pda);
    build_and_send_tx(svm, vec![create_ix], authority, vec![]);
    user_pda
}

pub fn update_comm_key(
    svm: &mut LiteSVM,
    authority: &Keypair,
    admin_pda: Pubkey,
    new_comm_key: Pubkey,
) {
    let update_ix = ix_update_comm_key(authority, admin_pda, new_comm_key);
    build_and_send_tx(svm, vec![update_ix], authority, vec![]);
}

pub fn close_profile(svm: &mut LiteSVM, authority: &Keypair, admin_pda: Pubkey) {
    let close_ix = ix_close_profile(authority, admin_pda);
    build_and_send_tx(svm, vec![close_ix], authority, vec![]);
}

pub fn deposit(svm: &mut LiteSVM, authority: &Keypair, admin_pda: Pubkey, amount: u64) {
    let deposit_ix = ix_deposit(authority, admin_pda, amount);
    build_and_send_tx(svm, vec![deposit_ix], authority, vec![]);
}

pub fn withdraw(
    svm: &mut LiteSVM,
    authority: &Keypair,
    admin_pda: Pubkey,
    destination: Pubkey,
    amount: u64,
) {
    let withdraw_ix = ix_withdraw(authority, admin_pda, destination, amount);
    build_and_send_tx(svm, vec![withdraw_ix], authority, vec![]);
}

use solana_program::sysvar::instructions;
use solana_sdk::signer::Signer;
use std::convert::TryInto;

pub struct DispatchCommandArgs {
    pub command_id: u16,
    pub price: u64,
    pub timestamp: i64,
    pub payload: Vec<u8>,
}

pub fn dispatch_command(
    svm: &mut LiteSVM,
    authority: &Keypair,
    admin_pda: Pubkey,
    oracle: &Keypair,
    args: DispatchCommandArgs,
) {
    // 1. Construct the message the oracle needs to sign
    let message = [
        args.command_id.to_le_bytes().as_ref(),
        args.price.to_le_bytes().as_ref(),
        args.timestamp.to_le_bytes().as_ref(),
    ]
    .concat();

    // 2. Sign the message and create the Ed25519 signature verification instruction
    let signature = oracle.sign_message(&message);
    let pubkey_bytes = oracle.pubkey().to_bytes();
    let signature_bytes: [u8; 64] = signature.as_ref().try_into().unwrap();
    let ed25519_ix = solana_sdk::ed25519_instruction::new_ed25519_instruction_with_signature(
        &message,
        &signature_bytes,
        &pubkey_bytes,
    );

    // 3. Create the actual dispatch command instruction
    let dispatch_ix = ix_dispatch_command(
        authority,
        admin_pda,
        args.command_id,
        args.price,
        args.timestamp,
        args.payload,
    );

    // 4. Send both instructions in the same transaction
    build_and_send_tx(svm, vec![ed25519_ix, dispatch_ix], authority, vec![]);
}

pub fn request_unban(svm: &mut LiteSVM, authority: &Keypair, admin_pda: Pubkey) {
    let ix = ix_request_unban(authority, admin_pda);
    build_and_send_tx(svm, vec![ix], authority, vec![]);
}

pub fn ix_create_profile(
    authority: &Keypair,
    communication_pubkey: Pubkey,
    target_admin_pda: Pubkey,
) -> (Instruction, Pubkey) {
    let (user_pda, _) = Pubkey::find_program_address(
        &[
            b"user",
            authority.pubkey().as_ref(),
            target_admin_pda.as_ref(),
        ],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::UserCreateProfile {
        target_admin_pda,
        communication_pubkey,
    }
    .data();

    let accounts = w3b2_accounts::UserCreateProfile {
        authority: authority.pubkey(),
        admin_profile: target_admin_pda,
        user_profile: user_pda,
        system_program: system_program::ID,
    }
    .to_account_metas(None);

    (
        Instruction {
            program_id: w3b2_solana_program::ID,
            accounts,
            data,
        },
        user_pda,
    )
}

pub fn ix_update_comm_key(authority: &Keypair, admin_pda: Pubkey, new_key: Pubkey) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::UserUpdateCommKey { new_key }.data();

    let accounts = w3b2_accounts::UserUpdateCommKey {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
        user_profile: user_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    }
}

pub fn ix_close_profile(authority: &Keypair, admin_pda: Pubkey) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::UserCloseProfile {}.data();

    let accounts = w3b2_accounts::UserCloseProfile {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
        user_profile: user_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    }
}

pub fn ix_deposit(authority: &Keypair, admin_pda: Pubkey, amount: u64) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::UserDeposit { amount }.data();

    let accounts = w3b2_accounts::UserDeposit {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
        user_profile: user_pda,
        system_program: system_program::ID,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    }
}

pub fn ix_withdraw(
    authority: &Keypair,
    admin_pda: Pubkey,
    destination: Pubkey,
    amount: u64,
) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::UserWithdraw { amount }.data();

    let accounts = w3b2_accounts::UserWithdraw {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
        user_profile: user_pda,
        destination,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    }
}

pub fn ix_dispatch_command(
    authority: &Keypair,
    admin_pda: Pubkey,
    command_id: u16,
    price: u64,
    timestamp: i64,
    payload: Vec<u8>,
) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::UserDispatchCommand {
        command_id,
        price,
        timestamp,
        payload,
    }
    .data();

    let accounts = w3b2_accounts::UserDispatchCommand {
        authority: authority.pubkey(),
        user_profile: user_pda,
        admin_profile: admin_pda,
        instructions: instructions::ID,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    }
}

pub fn ix_request_unban(authority: &Keypair, admin_pda: Pubkey) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::UserRequestUnban {}.data();

    let accounts = w3b2_accounts::UserRequestUnban {
        authority: authority.pubkey(),
        user_profile: user_pda,
        admin_profile: admin_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    }
}

/// A flexible helper to build a `UserDispatchCommand` instruction with explicit PDAs.
/// This is useful for failure tests where we need to provide mismatched accounts.
pub fn build_dispatch_command_instruction(
    authority_pubkey: &Pubkey,
    user_profile_pda: Pubkey,
    admin_profile_pda: Pubkey,
    command_id: u16,
    price: u64,
    timestamp: i64,
    payload: Vec<u8>,
) -> Instruction {
    let data = w3b2_instruction::UserDispatchCommand {
        command_id,
        price,
        timestamp,
        payload,
    }
    .data();

    let accounts = w3b2_accounts::UserDispatchCommand {
        authority: *authority_pubkey,
        user_profile: user_profile_pda,
        admin_profile: admin_profile_pda,
        instructions: instructions::ID,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    }
}
