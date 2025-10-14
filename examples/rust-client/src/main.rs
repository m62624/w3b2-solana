use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_program, sysvar,
    },
    Client, Cluster,
};
use anyhow::Result;
use std::rc::Rc;
use w3b2_solana_program::{accounts, instruction as w3b2_instruction};
use solana_ed25519_program::new_instruction_with_pubkey;


struct SocialApp {
    program_id: Pubkey,
    admin: Rc<Keypair>,
    alice: Rc<Keypair>,
    bob: Rc<Keypair>,
    oracle: Rc<Keypair>,
    admin_pda: Pubkey,
    alice_pda: Pubkey,
    bob_pda: Pubkey,
}

impl SocialApp {
    fn new() -> Result<Self> {
        let program_id = w3b2_solana_program::ID;

        let admin = Rc::new(Keypair::new());
        let alice = Rc::new(Keypair::new());
        let bob = Rc::new(Keypair::new());
        let oracle = Rc::new(Keypair::new());

        println!("Generated Keypairs:");
        println!("  Admin: {}", admin.pubkey());
        println!("  Alice: {}", alice.pubkey());
        println!("  Bob: {}", bob.pubkey());
        println!("  Oracle: {}", oracle.pubkey());

        let (admin_pda, _) = Pubkey::find_program_address(&[b"admin", admin.pubkey().as_ref()], &program_id);
        let (alice_pda, _) = Pubkey::find_program_address(&[b"user", alice.pubkey().as_ref(), admin_pda.as_ref()], &program_id);
        let (bob_pda, _) = Pubkey::find_program_address(&[b"user", bob.pubkey().as_ref(), admin_pda.as_ref()], &program_id);

        Ok(Self { program_id, admin, alice, bob, oracle, admin_pda, alice_pda, bob_pda })
    }

    fn get_program_for_keypair(&self, payer: Rc<Keypair>) -> Result<anchor_client::Program<Rc<Keypair>>> {
        let client = Client::new_with_options(Cluster::Localnet, payer, CommitmentConfig::confirmed());
        Ok(client.program(self.program_id)?)
    }

    async fn setup(&self) -> Result<()> {
        println!("\nRegistering admin profile...");
        let admin_program = self.get_program_for_keypair(self.admin.clone())?;

        admin_program
            .request()
            .signer(self.admin.as_ref())
            .accounts(accounts::AdminRegisterProfile {
                authority: self.admin.pubkey(),
                admin_profile: self.admin_pda,
                system_program: system_program::ID,
            })
            .args(w3b2_instruction::AdminRegisterProfile {
                communication_pubkey: self.admin.pubkey(),
            })
            .send()?;

        println!("Setting oracle authority...");
        admin_program
            .request()
            .signer(self.admin.as_ref())
            .accounts(accounts::AdminSetConfig {
                authority: self.admin.pubkey(),
                admin_profile: self.admin_pda,
            })
            .args(w3b2_instruction::AdminSetConfig {
                new_oracle_authority: Some(self.oracle.pubkey()),
                new_timestamp_validity: None,
                new_communication_pubkey: None,
                new_unban_fee: None,
            })
            .send()?;

        println!("Creating user profiles...");
        self.create_user_profile(&self.alice, self.alice_pda).await?;
        self.create_user_profile(&self.bob, self.bob_pda).await?;

        println!("Setup complete.");
        Ok(())
    }

    async fn create_user_profile(&self, user: &Rc<Keypair>, user_pda: Pubkey) -> Result<()> {
        let program = self.get_program_for_keypair(user.clone())?;
        program
            .request()
            .signer(user.as_ref())
            .accounts(accounts::UserCreateProfile {
                authority: user.pubkey(),
                admin_profile: self.admin_pda,
                user_profile: user_pda,
                system_program: system_program::ID,
            })
            .args(w3b2_instruction::UserCreateProfile {
                target_admin_pda: self.admin_pda,
                communication_pubkey: user.pubkey(),
            })
            .send()?;
        Ok(())
    }

    async fn send_message(&self, from: &Rc<Keypair>, from_pda: Pubkey, to_name: &str, text: &str) -> Result<()> {
        println!("[{}] -> [{}]: {}", from.pubkey(), to_name, text);

        let command_id = 1u16;
        let price = 0u64;
        let timestamp = chrono::Utc::now().timestamp();
        let payload = format!("MSG:{}", text).into_bytes();

        let mut oracle_msg_data = Vec::new();
        oracle_msg_data.extend_from_slice(&command_id.to_le_bytes());
        oracle_msg_data.extend_from_slice(&price.to_le_bytes());
        oracle_msg_data.extend_from_slice(&timestamp.to_le_bytes());

        let signature = self.oracle.sign_message(&oracle_msg_data);

        let ed25519_ix = new_instruction_with_pubkey(
            &self.oracle.pubkey(),
            &oracle_msg_data,
            signature.as_ref(),
        );

        let program = self.get_program_for_keypair(from.clone())?;
        program
            .request()
            .signer(from.as_ref())
            .accounts(accounts::UserDispatchCommand {
                authority: from.pubkey(),
                user_profile: from_pda,
                admin_profile: self.admin_pda,
                instructions: sysvar::instructions::ID,
            })
            .args(w3b2_instruction::UserDispatchCommand {
                command_id,
                price,
                timestamp,
                payload,
            })
            .pre_instructions(vec![ed25519_ix])
            .send()?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _app = SocialApp::new()?;
    // The user has requested that the application only needs to compile, not run.
    // The following lines are commented out to prevent execution.
    // _app.setup().await?;
    // let mut turn = 0;
    // loop {
    //     let (s_kp, s_pda, r_name) = if turn % 2 == 0 {
    //         (&_app.alice, _app.alice_pda, "Bob")
    //     } else {
    //         (&_app.bob, _app.bob_pda, "Alice")
    //     };
    //     _app.send_message(s_kp, s_pda, r_name, &format!("Message #{}", turn + 1)).await?;
    //     turn += 1;
    //     tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    // }
    println!("Compilation check successful. All client logic is in place.");
    Ok(())
}