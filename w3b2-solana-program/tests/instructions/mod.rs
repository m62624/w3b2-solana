#![allow(dead_code)]

pub mod admin;
pub mod log;
pub mod user;

use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use base64::{engine::general_purpose, Engine as _};
use litesvm::types::{FailedTransactionMetadata, TransactionMetadata};
use litesvm::LiteSVM;
use solana_program::clock::Clock;

use solana_program::{instruction::Instruction, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use w3b2_solana_program::{accounts as w3b2_accounts, instruction as w3b2_instruction};

/// A constant path to the compiled on-chain program binary (`.so` file).
/// This is used by `setup_svm` to load the program into the test environment.
const PATH_SBF: &str = "../target/deploy/w3b2_solana_program.so";

/// Initializes the `LiteSVM` test environment and loads the W3B2 Bridge program into it.
/// This function serves as the foundation for every test case, creating a fresh,
/// sandboxed "virtual blockchain" for each test to run in.
///
/// # Returns
/// A new instance of `LiteSVM` with the program successfully loaded.
pub fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(w3b2_solana_program::ID, PATH_SBF)
        .unwrap();
    // Initialize the Clock sysvar, as the program depends on it for timestamps.
    svm.set_sysvar(&Clock::default());
    svm
}

/// A simple wrapper for `Keypair::new()` for consistency across tests.
///
/// # Returns
/// A new, randomly generated `Keypair`.
pub fn create_keypair() -> Keypair {
    Keypair::new()
}

/// Creates a new `Keypair` and funds its on-chain account with a specified amount of lamports.
/// This is essential for creating `authority` or `payer` accounts (wallets) that need
/// to sign transactions and pay for fees and rent.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment where the airdrop will occur.
/// * `lamports` - The amount of lamports to airdrop to the new keypair's account.
///
/// # Returns
/// The new, funded `Keypair`.
pub fn create_funded_keypair(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports).unwrap();
    keypair
}

/// A generic helper to construct, sign, and send a transaction to the `LiteSVM`.
/// This function is the workhorse for executing instructions in the test environment.
/// It automatically includes a `ComputeBudget` instruction and handles signing from
/// multiple keypairs.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `instructions` - A vector of `Instruction`s to be included in the transaction.
/// * `payer_and_signer` - The primary `Keypair` (the user's or admin's wallet) that will
///   both sign the transaction and pay for the associated fees.
/// * `additional_signers` - A vector of other `Keypair`s that are required to sign
///   the transaction, if any.
pub fn build_and_send_tx(
    svm: &mut LiteSVM,
    instructions: Vec<Instruction>,
    payer_and_signer: &Keypair,
    additional_signers: Vec<&Keypair>,
) -> Vec<String> {
    let mut signers = vec![payer_and_signer];
    signers.extend(additional_signers);

    let mut all_instructions = vec![ComputeBudgetInstruction::set_compute_unit_limit(400_000)];
    all_instructions.extend(instructions);

    let mut tx = Transaction::new_with_payer(&all_instructions, Some(&payer_and_signer.pubkey()));

    tx.sign(&signers, svm.latest_blockhash());

    // Advance the clock to simulate time passing between transactions.
    let mut clock = svm.get_sysvar::<Clock>();
    clock.slot += 1;
    svm.set_sysvar(&clock);

    let result = svm.send_transaction(tx).expect("Transaction failed");

    result.logs
}

pub fn parse_events<E>(logs: &[String]) -> Vec<E>
where
    E: anchor_lang::Event + anchor_lang::AnchorDeserialize + anchor_lang::Discriminator,
{
    let mut events = Vec::new();
    for log in logs {
        if let Some(data_str) = log.strip_prefix("Program data: ") {
            if let Ok(bytes) = general_purpose::STANDARD.decode(data_str.trim()) {
                if bytes.len() > E::DISCRIMINATOR.len() {
                    let (disc_bytes, event_data) = bytes.split_at(E::DISCRIMINATOR.len());
                    if disc_bytes == E::DISCRIMINATOR {
                        if let Ok(e) = E::try_from_slice(event_data) {
                            events.push(e);
                        }
                    }
                }
            }
        }
    }
    events
}

/// Extracts the custom program error code from a transaction error.
/// This is used in failure-case tests to assert that the correct error was returned.
pub fn get_error_code(
    result: Result<TransactionMetadata, FailedTransactionMetadata>,
) -> Option<u32> {
    match result {
        Err(failed_meta) => match failed_meta.err {
            solana_sdk::transaction::TransactionError::InstructionError(
                _,
                solana_sdk::instruction::InstructionError::Custom(code),
            ) => Some(code),
            _ => None,
        },
        _ => {
            println!("Unexpected transaction result: {result:?}");
            None
        }
    }
}

/// Sets up a standard test scenario with one admin and one user profile.
pub fn setup_profiles(
    svm: &mut litesvm::LiteSVM,
) -> (
    solana_sdk::signature::Keypair,
    solana_sdk::pubkey::Pubkey,
    solana_sdk::signature::Keypair,
    solana_sdk::pubkey::Pubkey,
) {
    let admin_authority = create_funded_keypair(svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(svm, &admin_authority, create_keypair().pubkey());

    let user_authority = create_funded_keypair(svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(svm, &user_authority, create_keypair().pubkey(), admin_pda);

    (admin_authority, admin_pda, user_authority, user_pda)
}
