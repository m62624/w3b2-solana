#[allow(deprecated)]
use borsh::{BorshDeserialize, BorshSerialize};
use lazy_static::lazy_static;
use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_program::system_program;
use solana_sdk::{
    instruction::AccountMeta, instruction::Instruction, message::Message,
    message::VersionedMessage, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::VersionedTransaction,
};

const PATH_SBF: &str = "../target/deploy/w3b2_bridge_program.so";

lazy_static! {
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

#[derive(BorshSerialize, BorshDeserialize)]
struct RegisterAdminArgs;

fn make_register_admin_ix(program_id: &Pubkey, authority: &Pubkey, payer: &Pubkey) -> Instruction {
    let (_, bump) = Pubkey::find_program_address(&[b"admin", authority.as_ref()], program_id);
    let admin_pda =
        Pubkey::create_program_address(&[b"admin", authority.as_ref(), &[bump]], program_id)
            .expect("PDA derivation failed");
    let accounts = vec![
        AccountMeta::new(admin_pda, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    let data = anchor_discriminator("register_admin").to_vec(); // пустой args
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

#[test]
fn test_register_admin_success() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    let authority = Keypair::new();

    svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();

    let ix = make_register_admin_ix(&PROGRAM_ID, &authority.pubkey(), &payer.pubkey());
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &authority])
        .unwrap();

    svm.send_transaction(tx)
        .expect("register_admin should succeed");

    let (_, bump) =
        Pubkey::find_program_address(&[b"admin", authority.pubkey().as_ref()], &PROGRAM_ID);
    let admin_pda = Pubkey::create_program_address(
        &[b"admin", authority.pubkey().as_ref(), &[bump]],
        &PROGRAM_ID,
    )
    .unwrap();
    let acc = svm.get_account(&admin_pda).unwrap();
    assert_eq!(acc.data[8..40], authority.pubkey().to_bytes()); // owner pubkey stored
}

#[test]
fn test_register_admin_already_registered() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    let authority = Keypair::new();

    svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();

    let ix = make_register_admin_ix(&PROGRAM_ID, &authority.pubkey(), &payer.pubkey());
    let blockhash = svm.latest_blockhash();
    let msg1 = Message::new_with_blockhash(&[ix.clone()], Some(&payer.pubkey()), &blockhash);
    let tx1 = VersionedTransaction::try_new(VersionedMessage::Legacy(msg1), &[&payer, &authority])
        .unwrap();
    svm.send_transaction(tx1)
        .expect("first register_admin should succeed");

    // second registration -> should fail
    let msg2 = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx2 = VersionedTransaction::try_new(VersionedMessage::Legacy(msg2), &[&payer, &authority])
        .unwrap();
    let res = svm.send_transaction(tx2);
    assert!(res.is_err());
}

#[test]
fn test_register_admin_wrong_signer() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    let authority = Keypair::new();
    let attacker = Keypair::new();

    svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let ix = make_register_admin_ix(&PROGRAM_ID, &authority.pubkey(), &payer.pubkey());
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);

    // пытаемся подписать транзакцию "неправильным" authority
    let tx_result =
        VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &attacker]);
    assert!(
        tx_result.is_err(),
        "Transaction creation should fail due to signer mismatch"
    );
}
