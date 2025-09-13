use anchor_client::{
    solana_sdk::{
        signature::{read_keypair_file, Keypair, Signer},
        system_program,
    },
    Client,
};
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::UiTransactionEncoding;
use std::rc::Rc;
use std::time::{Duration, Instant};
use w3b2_bridge_program::{self};

fn print_logs(rpc: &RpcClient, sig: &solana_sdk::signature::Signature) -> Result<()> {
    let tx = rpc.get_transaction(sig, UiTransactionEncoding::Json)?;
    if let Some(meta) = tx.transaction.meta {
        if let Some(logs) = Option::<Vec<String>>::from(meta.log_messages) {
            println!("--- Logs for {} ---", sig);
            for l in logs {
                println!("{}", l);
            }
        }
    }
    Ok(())
}

#[test]
fn admin_request() -> Result<()> {
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = RpcClient::new(rpc_url.clone());

    // payer — тот же, что и при deploy (id.json)
    let payer_kp = read_keypair_file(dirs::home_dir().unwrap().join(".config/solana/id.json"))
        .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {}", e))?;
    let payer = Rc::new(payer_kp);

    println!("Requesting airdrop for payer: {}", payer.pubkey());
    rpc.request_airdrop(&payer.pubkey(), 2_000_000_000)?; // 2 SOL

    // ждём подтверждения airdrop (тут 30s — запас)
    let start = Instant::now();
    loop {
        let balance = rpc.get_balance(&payer.pubkey())?;
        if balance > 0 {
            println!("Airdrop confirmed! Balance: {} lamports", balance);
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            return Err(anyhow::anyhow!("Airdrop confirmation timed out."));
        }
        std::thread::sleep(Duration::from_millis(300));
    }

    let client = Client::new(anchor_client::Cluster::Localnet, payer.clone());
    let program_id = w3b2_bridge_program::ID;
    let program = client
        .program(program_id)
        .expect("failed to create program client");

    println!("Using payer: {}", payer.pubkey());
    println!("Program ID: {}", program_id);

    // новая authority — каждый запуск свой ключ -> уникальный PDA
    let authority = Keypair::new();

    let (admin_profile, _bump) =
        Pubkey::find_program_address(&[b"admin", authority.pubkey().as_ref()], &program_id);

    println!(
        "Registering admin PDA: {} for authority {}",
        admin_profile,
        authority.pubkey()
    );

    // подписываем и payer, и authority (anchor требует signer для authority)
    let tx_register = program
        .request()
        .accounts(w3b2_bridge_program::accounts::RegisterAdmin {
            admin_profile,
            payer: payer.pubkey(),
            authority: authority.pubkey(),
            system_program: system_program::ID,
        })
        .args(w3b2_bridge_program::instruction::RegisterAdmin {
            funding_amount: 1_000_000_000u64,
        })
        .signer(&*payer)
        .signer(&authority)
        .send()?;

    println!("RegisterAdmin OK: {}", tx_register);
    print_logs(&rpc, &tx_register)?;

    // Verify account exists on-chain (optional, но полезно)
    if let Ok(acc) = rpc.get_account(&admin_profile) {
        println!("Admin PDA exists, lamports: {}", acc.lamports);
    } else {
        println!("Warning: failed to fetch admin PDA account after tx.");
    }

    Ok(())
}
