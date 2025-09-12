// w3b2-bridge-program/tests/litesvm_integration.rs
use anchor_lang::{prelude::Pubkey as AnchorPubkey, system_program};
use borsh::{BorshDeserialize, BorshSerialize};
use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_message::Message;
use solana_message::VersionedMessage;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::transaction::VersionedTransaction;
use w3b2_common::{AccountType, CommandMode, UserAccount};

fn to_anchor_pubkey(pubkey: &Pubkey) -> AnchorPubkey {
    AnchorPubkey::new_from_array(pubkey.to_bytes())
}

use solana_sdk::instruction::AccountMeta;

lazy_static::lazy_static! {
    static ref PROGRAM_ID: Pubkey = Pubkey::new_from_array(w3b2_bridge_program::id().to_bytes());
}

fn anchor_discriminator(method: &str) -> [u8; 8] {
    let mut h = Sha256::new();
    h.update(format!("global:{}", method).as_bytes());
    let out = h.finalize();
    let mut d = [0u8; 8];
    d.copy_from_slice(&out[..8]);
    d
}

fn anchor_instruction_data<T: BorshSerialize>(method: &str, args: &T) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&anchor_discriminator(method));
    args.serialize(&mut data).expect("borsh serialize");
    data
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct RegisterUserArgs {
    account_type: AccountType,
    linked_wallet: Option<[u8; 32]>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct DispatchCommandArgs {
    command_id: u64,
    mode: CommandMode,
    payload: Vec<u8>,
}

fn user_pda_for(authority: &Pubkey) -> (Pubkey, u8) {
    let authority_anchor = AnchorPubkey::new_from_array(authority.to_bytes());
    let (pda, bump) = AnchorPubkey::find_program_address(
        &[b"user", authority_anchor.as_ref()],
        &w3b2_bridge_program::id(),
    );
    (Pubkey::new_from_array(pda.to_bytes()), bump)
}

#[derive(Debug, BorshDeserialize)]
struct UserPdaData {
    profile: UserAccount,
    linked_wallet: Option<[u8; 32]>,
    created_at: u64,
}

fn make_register_user_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    payer: &Pubkey,
    account_type: AccountType,
    linked_wallet: Option<[u8; 32]>,
) -> Instruction {
    let (user_pda, _) = user_pda_for(authority);
    let accounts = vec![
        AccountMeta::new(user_pda, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    let args = RegisterUserArgs {
        account_type,
        linked_wallet,
    };
    let data = anchor_instruction_data("register_user", &args);
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

fn make_dispatch_command_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    command_id: u64,
    mode: CommandMode,
    payload: Vec<u8>,
) -> Instruction {
    let (user_pda, _) = user_pda_for(authority);
    let accounts = vec![
        AccountMeta::new(user_pda, false),
        AccountMeta::new_readonly(*authority, true),
    ];
    let args = DispatchCommandArgs {
        command_id,
        mode,
        payload,
    };
    let data = anchor_instruction_data("dispatch_command", &args);
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

fn deserialize_user_pda(data: &[u8]) -> Option<UserPdaData> {
    if data.len() < 8 {
        return None;
    }
    let mut slice = &data[8..];
    UserPdaData::deserialize(&mut slice).ok()
}

#[test]
fn test_register_user_success() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    let authority = Keypair::new();

    // Загружаем программу в VM
    let program_id = *PROGRAM_ID;
    svm.add_program_from_file(program_id, "../../target/deploy/w3b2_bridge_program.so")
        .expect("add program");

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();

    let ix = make_register_user_ix(
        &program_id,
        &authority.pubkey(),
        &payer.pubkey(),
        AccountType::ExistingWallet,
        Some(authority.pubkey().to_bytes()),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &authority])
        .unwrap();

    svm.send_transaction(tx).expect("tx send");

    let (user_pda, _) = user_pda_for(&authority.pubkey());
    let acc = svm.get_account(&user_pda).unwrap();
    let parsed = deserialize_user_pda(&acc.data).expect("deserialize user pda");
    assert_eq!(parsed.profile.owner, authority.pubkey().to_bytes());
}
