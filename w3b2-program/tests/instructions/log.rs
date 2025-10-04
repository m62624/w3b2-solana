// tests/instructions/log.rs

use super::*;

pub fn log_action(
    svm: &mut LiteSVM,
    authority: &Keypair,
    user_profile_pda: Pubkey,
    admin_profile_pda: Pubkey,
    session_id: u64,
    action_code: u16,
) -> Vec<String> {
    let log_ix = ix_log_action(
        authority,
        user_profile_pda,
        admin_profile_pda,
        session_id,
        action_code,
    );
    build_and_send_tx(svm, vec![log_ix], authority, vec![])
}

pub fn ix_log_action(
    authority: &Keypair,
    user_profile_pda: Pubkey,
    admin_profile_pda: Pubkey,
    session_id: u64,
    action_code: u16,
) -> Instruction {
    let data = w3b2_instruction::LogAction {
        session_id,
        action_code,
    }
    .data();
    let accounts = w3b2_accounts::LogAction {
        authority: authority.pubkey(),
        user_profile: user_profile_pda,
        admin_profile: admin_profile_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_program::ID,
        accounts,
        data,
    }
}
