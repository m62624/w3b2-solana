#![allow(deprecated)]

use anchor_lang::InstructionData;
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use borsh::{BorshDeserialize, BorshSerialize};
use lazy_static::lazy_static;
use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_program::rent::Rent;
use solana_program::system_program;
use solana_sdk::{
    instruction::AccountMeta, instruction::Instruction, message::Message,
    message::VersionedMessage, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::VersionedTransaction,
};
use w3b2_bridge_program::sm_accounts::FundingRequest;
use w3b2_bridge_program::types::FundingStatus;

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

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RegisterAdminArgs {
    pub funding_amount: u64,
}

fn make_register_admin_ix(
    program_id: &Pubkey,
    authority: &Pubkey,
    payer: &Pubkey,
    funding_amount: u64,
) -> Instruction {
    let (_, bump) = Pubkey::find_program_address(&[b"admin", authority.as_ref()], program_id);
    let admin_pda =
        Pubkey::create_program_address(&[b"admin", authority.as_ref(), &[bump]], program_id)
            .unwrap();

    let data = w3b2_bridge_program::instruction::RegisterAdmin { funding_amount }.data();

    let accounts = vec![
        AccountMeta::new(admin_pda, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

mod fn_register_admin {
    use super::*;

    #[test]
    fn test_register_admin_success() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let authority = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&authority.pubkey(), 0).unwrap();

        let funding_amount = 10000;
        let ix = make_register_admin_ix(
            &PROGRAM_ID,
            &authority.pubkey(),
            &payer.pubkey(),
            funding_amount,
        );
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
        let tx =
            VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &authority])
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

        // Проверяем, что owner записан корректно
        assert_eq!(acc.data[8..40], authority.pubkey().to_bytes());

        // Проверяем, что на PDA зачислено хотя бы rent + funding_amount
        let rent = Rent::default();
        let min_balance = rent.minimum_balance(acc.data.len()) + funding_amount;
        assert!(acc.lamports >= min_balance, "PDA balance too low");
    }

    #[test]
    fn test_register_admin_insufficient_balance() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let authority = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        // payer имеет недостаточно lamports
        svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&authority.pubkey(), 0).unwrap();

        let ix = make_register_admin_ix(
            &PROGRAM_ID,
            &authority.pubkey(),
            &payer.pubkey(),
            1_000_000_000,
        );
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
        let tx =
            VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &authority])
                .unwrap();

        let res = svm.send_transaction(tx);
        println!("Result: {:?}", res);
        assert!(
            res.is_err(),
            "Should fail due to insufficient payer balance"
        );
    }

    #[test]
    fn test_register_admin_already_registered() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let authority = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();

        let ix =
            make_register_admin_ix(&PROGRAM_ID, &authority.pubkey(), &payer.pubkey(), 1_000_000);
        let blockhash = svm.latest_blockhash();
        let msg1 = Message::new_with_blockhash(&[ix.clone()], Some(&payer.pubkey()), &blockhash);
        let tx1 =
            VersionedTransaction::try_new(VersionedMessage::Legacy(msg1), &[&payer, &authority])
                .unwrap();
        let res0 = svm
            .send_transaction(tx1)
            .expect("first register_admin should succeed");

        println!("First registration result: {:?}", res0);
        // Попытка повторной регистрации -> должна упасть
        let msg2 = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
        let tx2 =
            VersionedTransaction::try_new(VersionedMessage::Legacy(msg2), &[&payer, &authority])
                .unwrap();
        let res = svm.send_transaction(tx2);
        println!("Second registration result: {:?}", res);
        assert!(res.is_err(), "Second registration should fail");
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

        let ix =
            make_register_admin_ix(&PROGRAM_ID, &authority.pubkey(), &payer.pubkey(), 1_000_000);
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
}

#[derive(BorshSerialize, BorshDeserialize)]
struct RequestFundingArgs {
    amount: u64,
    target_admin: Pubkey,
}

/// Derive PDA for funding request
fn funding_request_pda_for(user_wallet: &Pubkey, payer: &Pubkey) -> (Pubkey, u8) {
    let seeds = [b"funding", user_wallet.as_ref(), &payer.to_bytes()];
    Pubkey::find_program_address(&seeds, &*PROGRAM_ID)
}

/// Construct request_funding instruction
fn make_request_funding_ix(
    program_id: &Pubkey,
    user_wallet: &Pubkey,
    payer: &Pubkey,
    amount: u64,
    target_admin: Pubkey,
) -> Instruction {
    let (funding_pda, _) = funding_request_pda_for(user_wallet, payer);
    let accounts = vec![
        AccountMeta::new(funding_pda, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*user_wallet, true),
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    let args = RequestFundingArgs {
        amount,
        target_admin,
    };
    let mut data = anchor_discriminator("request_funding").to_vec();
    data.extend_from_slice(&args.try_to_vec().unwrap());
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Deserialize FundingRequest PDA
fn try_deserialize(data: &[u8]) -> Option<FundingRequest> {
    if data.len() < 8 {
        return None;
    }
    FundingRequest::try_from_slice(&data[8..]).ok()
}

mod fn_request_funding {

    use super::*;

    #[test]
    fn test_request_funding_success() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let user = Keypair::new();
        let admin = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&user.pubkey(), 1_000_000).unwrap();
        svm.airdrop(&admin.pubkey(), 1_000_000_000).unwrap();

        // создаём админа
        let reg_admin_ix =
            make_register_admin_ix(&PROGRAM_ID, &admin.pubkey(), &payer.pubkey(), 1_000_000);
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[reg_admin_ix], Some(&payer.pubkey()), &blockhash);
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &admin])
            .unwrap();
        svm.send_transaction(tx).unwrap();

        // запрос финансирования
        let amount = 1_000_000;
        let req_ix = make_request_funding_ix(
            &PROGRAM_ID,
            &user.pubkey(),
            &payer.pubkey(),
            amount,
            admin.pubkey(),
        );
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[req_ix], Some(&payer.pubkey()), &blockhash);
        let tx =
            VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &user]).unwrap();
        svm.send_transaction(tx).unwrap();

        // проверяем FundingRequest PDA
        let (funding_pda, _) = funding_request_pda_for(&user.pubkey(), &payer.pubkey());
        let acc = svm.get_account(&funding_pda).unwrap();
        let parsed: FundingRequest = try_deserialize(&acc.data).unwrap();
        assert_eq!(parsed.user_wallet, user.pubkey());
        assert_eq!(parsed.target_admin, admin.pubkey());
        assert_eq!(parsed.amount, amount);
        assert_eq!(parsed.status, FundingStatus::Pending as u8);
    }

    #[test]
    fn test_request_funding_duplicate() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let user = Keypair::new();
        let admin = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&admin.pubkey(), 1_000_000_000).unwrap();

        // создаём админа
        let reg_admin_ix =
            make_register_admin_ix(&PROGRAM_ID, &admin.pubkey(), &payer.pubkey(), 1_000_000);
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[reg_admin_ix], Some(&payer.pubkey()), &blockhash);
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &admin])
            .unwrap();
        svm.send_transaction(tx).unwrap();

        // первый запрос финансирования
        let req_ix1 = make_request_funding_ix(
            &PROGRAM_ID,
            &user.pubkey(),
            &payer.pubkey(),
            1_000_000,
            admin.pubkey(),
        );
        let blockhash = svm.latest_blockhash();
        let msg1 =
            Message::new_with_blockhash(&[req_ix1.clone()], Some(&payer.pubkey()), &blockhash);
        let tx1 = VersionedTransaction::try_new(VersionedMessage::Legacy(msg1), &[&payer, &user])
            .unwrap();
        svm.send_transaction(tx1).unwrap();

        // второй запрос -> должен упасть (PDA уже существует)
        let msg2 = Message::new_with_blockhash(&[req_ix1], Some(&payer.pubkey()), &blockhash);
        let tx2 = VersionedTransaction::try_new(VersionedMessage::Legacy(msg2), &[&payer, &user])
            .unwrap();
        let res = svm.send_transaction(tx2);
        println!("Result: {:?}", res);
        assert!(res.is_err(), "Duplicate funding request should fail");
    }
}

pub fn make_approve_funding_ix(
    program_id: &Pubkey,
    funding_request: &Pubkey,
    admin_authority: &Pubkey,
    user_wallet: &Pubkey,
) -> Instruction {
    let (_, bump) = Pubkey::find_program_address(&[b"admin", admin_authority.as_ref()], program_id);
    let admin_pda =
        Pubkey::create_program_address(&[b"admin", admin_authority.as_ref(), &[bump]], program_id)
            .expect("PDA derivation failed");

    let accounts = vec![
        // 1. admin_profile: mutable, не signer
        AccountMeta::new(admin_pda, false),
        // 2. funding_request: mutable, не signer
        AccountMeta::new(*funding_request, false),
        // 3. user_wallet: mutable, не signer
        AccountMeta::new(*user_wallet, false),
        // 4. admin_authority: не mutable, но signer
        AccountMeta::new_readonly(*admin_authority, true),
        // 5. system_program: не mutable, не signer
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let data = anchor_discriminator("approve_funding").to_vec();
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

mod fn_approve_funding {
    use super::*;

    #[test]
    fn test_approve_funding_success() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let user = Keypair::new();
        let admin = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&admin.pubkey(), 1_000_000_000).unwrap();

        // создаём админа
        let reg_admin_ix =
            make_register_admin_ix(&PROGRAM_ID, &admin.pubkey(), &payer.pubkey(), 1_000_000);
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[reg_admin_ix], Some(&payer.pubkey()), &blockhash);
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &admin])
            .unwrap();
        svm.send_transaction(tx).unwrap();

        // запрос финансирования
        let amount = 1_000_000;
        let req_ix = make_request_funding_ix(
            &PROGRAM_ID,
            &user.pubkey(),
            &payer.pubkey(),
            amount,
            admin.pubkey(),
        );
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[req_ix], Some(&payer.pubkey()), &blockhash);
        let tx =
            VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &user]).unwrap();
        svm.send_transaction(tx).unwrap();

        // админ одобряет запрос
        let (funding_pda, _) = funding_request_pda_for(&user.pubkey(), &payer.pubkey());
        let approve_ix =
            make_approve_funding_ix(&PROGRAM_ID, &funding_pda, &admin.pubkey(), &user.pubkey());
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[approve_ix], Some(&admin.pubkey()), &blockhash);
        let tx = VersionedTransaction::try_new(
            VersionedMessage::Legacy(msg),
            &[&admin], // только admin подписывает
        )
        .unwrap();
        svm.send_transaction(tx).unwrap();

        // проверяем статус FundingRequest
        let acc = svm.get_account(&funding_pda).unwrap();
        let parsed: FundingRequest = try_deserialize(&acc.data).unwrap();
        println!("Parsed FundingRequest: {:?}", parsed);
        assert_eq!(parsed.status, FundingStatus::Approved as u8);
    }

    #[test]
    fn test_approve_funding_wrong_admin() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let user = Keypair::new();
        let admin = Keypair::new();
        let attacker = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&admin.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

        // создаём легитимного админа
        let reg_admin_ix =
            make_register_admin_ix(&PROGRAM_ID, &admin.pubkey(), &payer.pubkey(), 1_000_000);
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[reg_admin_ix], Some(&payer.pubkey()), &blockhash);
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &admin])
            .unwrap();
        svm.send_transaction(tx).unwrap();

        // создаём "чужого" админа (attacker) PDA, чтобы Anchor не ругался на AccountNotInitialized
        let reg_attacker_ix =
            make_register_admin_ix(&PROGRAM_ID, &attacker.pubkey(), &payer.pubkey(), 1_000_000);
        let blockhash = svm.latest_blockhash();
        let msg =
            Message::new_with_blockhash(&[reg_attacker_ix], Some(&payer.pubkey()), &blockhash);
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &attacker])
            .unwrap();
        svm.send_transaction(tx).unwrap();

        // создаём запрос финансирования для легитимного админа
        let req_ix = make_request_funding_ix(
            &PROGRAM_ID,
            &user.pubkey(),
            &payer.pubkey(),
            1_000_000,
            admin.pubkey(),
        );
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[req_ix], Some(&payer.pubkey()), &blockhash);
        let tx =
            VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &user]).unwrap();
        svm.send_transaction(tx).unwrap();

        // "чужой" админ пытается одобрить запрос
        let (funding_pda, _) = funding_request_pda_for(&user.pubkey(), &payer.pubkey());
        let approve_ix = make_approve_funding_ix(
            &PROGRAM_ID,
            &funding_pda,
            &attacker.pubkey(),
            &user.pubkey(),
        );
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[approve_ix], Some(&attacker.pubkey()), &blockhash);
        let tx =
            VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&attacker]).unwrap();
        let res = svm.send_transaction(tx);

        // Anchor теперь выдаст BridgeError::Unauthorized
        println!("Result: {:?}", res);
        assert!(res.is_err());
    }

    #[test]
    fn test_approve_funding_already_processed() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let user = Keypair::new();
        let admin = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&admin.pubkey(), 1_000_000_000).unwrap();

        // создаём админа и запрос
        let reg_admin_ix =
            make_register_admin_ix(&PROGRAM_ID, &admin.pubkey(), &payer.pubkey(), 1_000_000);
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[reg_admin_ix], Some(&payer.pubkey()), &blockhash);
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &admin])
            .unwrap();
        svm.send_transaction(tx).unwrap();

        let req_ix = make_request_funding_ix(
            &PROGRAM_ID,
            &user.pubkey(),
            &payer.pubkey(),
            1_000_000,
            admin.pubkey(),
        );
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[req_ix], Some(&payer.pubkey()), &blockhash);
        let tx =
            VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer, &user]).unwrap();
        svm.send_transaction(tx).unwrap();

        // первый одобряем
        let (funding_pda, _) = funding_request_pda_for(&user.pubkey(), &payer.pubkey());
        let approve_ix =
            make_approve_funding_ix(&PROGRAM_ID, &funding_pda, &admin.pubkey(), &user.pubkey());
        let blockhash = svm.latest_blockhash();
        let msg =
            Message::new_with_blockhash(&[approve_ix.clone()], Some(&admin.pubkey()), &blockhash);
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&admin]).unwrap();
        svm.send_transaction(tx).unwrap();

        // повторный одобряем -> должно падать
        let blockhash = svm.latest_blockhash();
        let msg =
            Message::new_with_blockhash(&[approve_ix.clone()], Some(&admin.pubkey()), &blockhash);
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&admin]).unwrap();
        let res = svm.send_transaction(tx);
        println!("Result: {:?}", res);
        assert!(res.is_err());
    }
}
