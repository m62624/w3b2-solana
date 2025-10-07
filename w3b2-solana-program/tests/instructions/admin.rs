#![allow(dead_code)]

use super::*;

pub fn create_profile(svm: &mut LiteSVM, authority: &Keypair, comm_key: Pubkey) -> Pubkey {
    let (register_ix, admin_pda) = ix_create_profile(authority, comm_key);
    build_and_send_tx(svm, vec![register_ix], authority, vec![]);
    admin_pda
}

pub fn update_comm_key(svm: &mut LiteSVM, authority: &Keypair, new_comm_key: Pubkey) {
    let update_ix = ix_update_comm_key(authority, new_comm_key);
    build_and_send_tx(svm, vec![update_ix], authority, vec![]);
}

pub fn close_profile(svm: &mut LiteSVM, authority: &Keypair) {
    let close_ix = ix_close_profile(authority);
    build_and_send_tx(svm, vec![close_ix], authority, vec![]);
}

pub fn set_oracle(svm: &mut LiteSVM, authority: &Keypair, new_oracle: Pubkey) {
    let set_oracle_ix = ix_set_oracle(authority, new_oracle);
    build_and_send_tx(svm, vec![set_oracle_ix], authority, vec![]);
}

pub fn withdraw(svm: &mut LiteSVM, authority: &Keypair, destination: Pubkey, amount: u64) {
    let withdraw_ix = ix_withdraw(authority, destination, amount);
    build_and_send_tx(svm, vec![withdraw_ix], authority, vec![]);
}

pub fn dispatch_command(
    svm: &mut LiteSVM,
    authority: &Keypair,
    user_profile_pda: Pubkey,
    command_id: u64,
    payload: Vec<u8>,
) {
    let dispatch_ix = ix_dispatch_command(authority, user_profile_pda, command_id, payload);
    build_and_send_tx(svm, vec![dispatch_ix], authority, vec![]);
}

fn ix_create_profile(authority: &Keypair, communication_pubkey: Pubkey) -> (Instruction, Pubkey) {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::AdminRegisterProfile {
        communication_pubkey,
    }
    .data();

    let accounts = w3b2_accounts::AdminRegisterProfile {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
        system_program: system_program::ID,
    }
    .to_account_metas(None);

    let ix = Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    };

    (ix, admin_pda)
}

pub fn ix_set_oracle(authority: &Keypair, new_oracle_authority: Pubkey) -> Instruction {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::AdminSetOracle {
        new_oracle_authority,
    }
    .data();

    let accounts = w3b2_accounts::AdminSetOracle {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    }
}

pub fn ix_update_comm_key(authority: &Keypair, new_key: Pubkey) -> Instruction {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::AdminUpdateCommKey { new_key }.data();

    let accounts = w3b2_accounts::AdminUpdateCommKey {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    }
}

pub fn ix_close_profile(authority: &Keypair) -> Instruction {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::AdminCloseProfile {}.data();

    let accounts = w3b2_accounts::AdminCloseProfile {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    }
}

pub fn ix_withdraw(authority: &Keypair, destination: Pubkey, amount: u64) -> Instruction {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::AdminWithdraw { amount }.data();

    let accounts = w3b2_accounts::AdminWithdraw {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
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
    user_profile_pda: Pubkey,
    command_id: u64,
    payload: Vec<u8>,
) -> Instruction {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_solana_program::ID,
    );

    let data = w3b2_instruction::AdminDispatchCommand {
        command_id,
        payload,
    }
    .data();

    let accounts = w3b2_accounts::AdminDispatchCommand {
        admin_authority: authority.pubkey(),
        admin_profile: admin_pda,
        user_profile: user_profile_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_solana_program::ID,
        accounts,
        data,
    }
}
