#![allow(dead_code)]

use anchor_lang::{prelude::Pubkey as AnchorPubkey, system_program};
use borsh::{BorshDeserialize, BorshSerialize};
use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_message::VersionedMessage;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::transaction::VersionedTransaction;
use w3b2_bridge_program::types::{CommandMode, UserAccount, WalletType};

const PATH_SBF: &str = "../target/deploy/w3b2_bridge_program.so";

lazy_static::lazy_static! {
    static ref PROGRAM_ID: Pubkey = Pubkey::new_from_array(w3b2_bridge_program::id().to_bytes());
}

/// Convert Solana Pubkey to Anchor Pubkey
fn to_anchor_pubkey(pubkey: &Pubkey) -> AnchorPubkey {
    AnchorPubkey::new_from_array(pubkey.to_bytes())
}

/// Compute Anchor discriminator for a method
fn anchor_discriminator(method: &str) -> [u8; 8] {
    let mut h = Sha256::new();
    h.update(format!("global:{}", method).as_bytes());
    let out = h.finalize();
    let mut d = [0u8; 8];
    d.copy_from_slice(&out[..8]);
    d
}

/// Serialize instruction data with Borsh
fn anchor_instruction_data<T: BorshSerialize>(method: &str, args: &T) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&anchor_discriminator(method));
    args.serialize(&mut data).expect("borsh serialize");
    data
}

/// PDA derivation
fn user_pda_for(authority: &Pubkey) -> (Pubkey, u8) {
    let authority_anchor = AnchorPubkey::new_from_array(authority.to_bytes());
    let (pda, bump) = AnchorPubkey::find_program_address(
        &[b"user", authority_anchor.as_ref()],
        &w3b2_bridge_program::id(),
    );
    (Pubkey::new_from_array(pda.to_bytes()), bump)
}

/// Borsh structs for instructions
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct RegisterUserArgs {
    account_type: WalletType,
    linked_wallet: Option<Pubkey>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct DispatchCommandArgs {
    command_id: u64,
    mode: CommandMode,
    payload: Vec<u8>,
}

/// Deserialize PDA data for verification
#[derive(Debug, BorshDeserialize)]
struct UserPdaData {
    profile: UserAccount,
    linked_wallet: Option<Pubkey>,
    created_at: u64,
}

fn deserialize_user_pda(data: &[u8]) -> Option<UserPdaData> {
    if data.len() < 8 {
        return None;
    }
    let mut slice = &data[8..];
    UserPdaData::deserialize(&mut slice).ok()
}

/// Construct register_user instruction
fn make_register_user_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    payer: &Pubkey,
    account_type: WalletType,
    linked_wallet: Option<Pubkey>,
) -> solana_sdk::instruction::Instruction {
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
    solana_sdk::instruction::Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Construct dispatch_command instruction
fn make_dispatch_command_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    command_id: u64,
    mode: CommandMode,
    payload: Vec<u8>,
) -> solana_sdk::instruction::Instruction {
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
    solana_sdk::instruction::Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

#[test]
fn test_register_user_success() {
    // Test: successful PDA registration
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    let authority = Keypair::new();

    svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();

    let ix = make_register_user_ix(
        &PROGRAM_ID,
        &authority.pubkey(),
        &payer.pubkey(),
        WalletType::ExistingWallet,
        Some(authority.pubkey()),
    );

    let blockhash = svm.latest_blockhash();
    let msg =
        solana_sdk::message::Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &authority])
        .unwrap();
    svm.send_transaction(tx).expect("transaction failed");

    let (user_pda, _) = user_pda_for(&authority.pubkey());
    let acc = svm.get_account(&user_pda).unwrap();
    let parsed = deserialize_user_pda(&acc.data).unwrap();
    assert_eq!(parsed.profile.owner, authority.pubkey());
    assert_eq!(parsed.profile.account_type, WalletType::ExistingWallet);
    assert_eq!(parsed.linked_wallet, Some(authority.pubkey()));
}

/// Test: prevent double registration
#[test]
fn test_register_user_already_registered() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    let authority = Keypair::new();
    svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();

    let ix1 = make_register_user_ix(
        &PROGRAM_ID,
        &authority.pubkey(),
        &payer.pubkey(),
        WalletType::ExistingWallet,
        None,
    );
    let ix2 = make_register_user_ix(
        &PROGRAM_ID,
        &authority.pubkey(),
        &payer.pubkey(),
        WalletType::NewWallet,
        None,
    );

    let blockhash = svm.latest_blockhash();
    let msg1 =
        solana_sdk::message::Message::new_with_blockhash(&[ix1], Some(&payer.pubkey()), &blockhash);
    let tx1 = VersionedTransaction::try_new(VersionedMessage::Legacy(msg1), &[&payer, &authority])
        .unwrap();
    svm.send_transaction(tx1)
        .expect("first registration failed");

    // second registration should fail
    let msg2 =
        solana_sdk::message::Message::new_with_blockhash(&[ix2], Some(&payer.pubkey()), &blockhash);
    let tx2 = VersionedTransaction::try_new(VersionedMessage::Legacy(msg2), &[&payer, &authority])
        .unwrap();
    let res = svm.send_transaction(tx2);
    println!("{:#?}", res);
    assert!(res.is_err());
}

/// Test: dispatch_command authorized owner
#[test]
fn test_dispatch_command_owner() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    let authority = Keypair::new();
    svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();

    // Register first
    let reg_ix = make_register_user_ix(
        &PROGRAM_ID,
        &authority.pubkey(),
        &payer.pubkey(),
        WalletType::ExistingWallet,
        None,
    );
    let blockhash = svm.latest_blockhash();
    let msg_reg = solana_sdk::message::Message::new_with_blockhash(
        &[reg_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );
    let tx_reg =
        VersionedTransaction::try_new(VersionedMessage::Legacy(msg_reg), &[&payer, &authority])
            .unwrap();
    svm.send_transaction(tx_reg).unwrap();

    // Dispatch command
    let payload = vec![1, 2, 3, 4];
    let cmd_ix = make_dispatch_command_ix(
        &PROGRAM_ID,
        &authority.pubkey(),
        42,
        CommandMode::OneWay,
        payload.clone(),
    );
    let msg_cmd = solana_sdk::message::Message::new_with_blockhash(
        &[cmd_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );
    let tx_cmd =
        VersionedTransaction::try_new(VersionedMessage::Legacy(msg_cmd), &[&payer, &authority])
            .unwrap();
    svm.send_transaction(tx_cmd).unwrap();

    // Verify PDA data unchanged
    let (user_pda, _) = user_pda_for(&authority.pubkey());
    let acc = svm.get_account(&user_pda).unwrap();
    let parsed = deserialize_user_pda(&acc.data).unwrap();
    assert_eq!(parsed.profile.owner, authority.pubkey());
    assert_eq!(parsed.linked_wallet, None);
}
