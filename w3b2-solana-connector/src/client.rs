use anchor_lang::{InstructionData, ToAccountMetas};
use async_trait::async_trait;
use solana_client::{client_error::ClientError, nonblocking::rpc_client::RpcClient};
use solana_ed25519_program::new_ed25519_instruction_with_signature;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::sysvar;
use solana_sdk::transaction::Transaction;
use solana_sdk::{hash::Hash, signature::Signature};
use std::sync::Arc;
use w3b2_solana_program::{accounts, instruction};

/// A trait abstracting over the asynchronous RPC client functionality needed by `TransactionBuilder`.
/// This allows for mocking and using different clients like `RpcClient` and `BanksClient`.
#[async_trait]
pub trait AsyncRpcClient: Send + Sync {
    async fn get_latest_blockhash(&self) -> Result<Hash, ClientError>;
    async fn send_and_confirm_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Signature, ClientError>;
}

#[async_trait]
impl AsyncRpcClient for RpcClient {
    async fn get_latest_blockhash(&self) -> Result<Hash, ClientError> {
        self.get_latest_blockhash().await
    }

    async fn send_and_confirm_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Signature, ClientError> {
        self.send_and_confirm_transaction(transaction).await
    }
}
/// A builder for preparing on-chain transactions for remote signing.
///
/// This struct provides methods to construct unsigned transactions for every
/// instruction in the W3B2 Bridge Program. It is designed for a non-custodial
/// architecture where the private key never leaves the client's device.
/// The server-side component (like a gRPC gateway) uses this builder to create
/// a transaction, sends it to the client for signing, and then receives the
/// signed transaction back for submission.
#[derive(Clone)]
pub struct TransactionBuilder<C: AsyncRpcClient + ?Sized> {
    /// A shared, thread-safe reference to the Solana JSON RPC client.
    rpc_client: Arc<C>,
}

impl<C> TransactionBuilder<C>
where
    C: AsyncRpcClient + ?Sized,
{
    /// Creates a new `TransactionBuilder`.
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - A shared client that implements `AsyncRpcClient` (e.g., `RpcClient` or `BanksClient`).
    pub fn new(rpc_client: Arc<C>) -> Self {
        Self { rpc_client }
    }

    /// Submits a fully signed transaction to the Solana network.
    ///
    /// This is the final step in the remote signing flow. After a client signs
    /// the transaction prepared by one of the `prepare_*` methods, the signed
    /// transaction is sent back to the server and submitted via this method.
    ///
    /// # Arguments
    ///
    /// * `transaction` - A `Transaction` object that has already been signed.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Signature` of the confirmed transaction.
    pub async fn submit_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Signature, ClientError> {
        self.rpc_client
            .send_and_confirm_transaction(transaction)
            .await
    }

    /// A private helper function to create a transaction from a vector of instructions.
    ///
    /// This function encapsulates the boilerplate of fetching the latest blockhash
    /// and creating a new transaction with a payer.
    async fn create_transaction_with_instructions(
        &self,
        payer: &Pubkey,
        instructions: Vec<Instruction>,
    ) -> Result<Transaction, ClientError> {
        let latest_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&instructions, Some(payer));
        tx.message.recent_blockhash = latest_blockhash;
        Ok(tx)
    }

    /// A private helper function to create a transaction from a single instruction.
    async fn create_transaction(
        &self,
        payer: &Pubkey,
        instruction: Instruction,
    ) -> Result<Transaction, ClientError> {
        self.create_transaction_with_instructions(payer, vec![instruction])
            .await
    }

    // --- Admin Transaction Preparations ---

    /// Prepares an `admin_register_profile` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The public key of the admin's wallet that will sign the transaction.
    /// * `communication_pubkey` - The public key for secure off-chain communication.
    pub async fn prepare_admin_register_profile(
        &self,
        authority: Pubkey,
        communication_pubkey: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_solana_program::ID);

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::AdminRegisterProfile {
                authority,
                admin_profile: admin_pda,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::AdminRegisterProfile {
                communication_pubkey,
            }
            .data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares an `admin_set_config` transaction.
    pub async fn prepare_admin_set_config(
        &self,
        authority: Pubkey,
        new_oracle_authority: Option<Pubkey>,
        new_timestamp_validity: Option<i64>,
        new_communication_pubkey: Option<Pubkey>,
    ) -> Result<Transaction, ClientError> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_solana_program::ID);

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::AdminSetConfig {
                authority,
                admin_profile: admin_pda,
            }
            .to_account_metas(None),
            data: instruction::AdminSetConfig {
                new_oracle_authority,
                new_timestamp_validity,
                new_communication_pubkey,
            }
            .data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares an `admin_withdraw` transaction.
    pub async fn prepare_admin_withdraw(
        &self,
        authority: Pubkey,
        amount: u64,
        destination: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_solana_program::ID);

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::AdminWithdraw {
                authority,
                admin_profile: admin_pda,
                destination,
            }
            .to_account_metas(None),
            data: instruction::AdminWithdraw { amount }.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares an `admin_close_profile` transaction.
    pub async fn prepare_admin_close_profile(
        &self,
        authority: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_solana_program::ID);

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::AdminCloseProfile {
                authority,
                admin_profile: admin_pda,
            }
            .to_account_metas(None),
            data: instruction::AdminCloseProfile {}.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares an `admin_dispatch_command` transaction.
    pub async fn prepare_admin_dispatch_command(
        &self,
        authority: Pubkey,
        target_user_profile_pda: Pubkey,
        command_id: u64,
        payload: Vec<u8>,
    ) -> Result<Transaction, ClientError> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_solana_program::ID);

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::AdminDispatchCommand {
                admin_authority: authority,
                admin_profile: admin_pda,
                user_profile: target_user_profile_pda,
            }
            .to_account_metas(None),
            data: instruction::AdminDispatchCommand {
                command_id,
                payload,
            }
            .data(),
        };

        self.create_transaction(&authority, ix).await
    }

    // --- User Transaction Preparations ---

    /// Prepares a `user_create_profile` transaction. The on-chain instruction
    /// requires the `admin_profile` PDA to be passed in the accounts list for verification.
    ///
    /// * `authority` - The user's wallet `Pubkey` that will sign and own the profile.
    /// * `target_admin_pda` - The `Pubkey` of the `AdminProfile` **PDA** to link to.
    /// * `communication_pubkey` - The user's public key for off-chain communication.
    pub async fn prepare_user_create_profile(
        &self,
        authority: Pubkey,
        target_admin_pda: Pubkey,
        communication_pubkey: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), target_admin_pda.as_ref()],
            &w3b2_solana_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::UserCreateProfile {
                authority,
                admin_profile: target_admin_pda,
                user_profile: user_pda,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::UserCreateProfile {
                target_admin_pda,
                communication_pubkey,
            }
            .data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares a `user_update_comm_key` transaction.
    ///
    /// * `authority` - The user's wallet `Pubkey`.
    /// * `admin_profile_pda` - The `Pubkey` of the `AdminProfile` **PDA** this user profile is linked to.
    /// * `new_key` - The new communication key to set.
    pub async fn prepare_user_update_comm_key(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
        new_key: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), admin_profile_pda.as_ref()],
            &w3b2_solana_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::UserUpdateCommKey {
                authority,
                user_profile: user_pda,
                admin_profile: admin_profile_pda,
            }
            .to_account_metas(None),
            data: instruction::UserUpdateCommKey { new_key }.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares a `user_deposit` transaction.
    ///
    /// * `authority` - The user's wallet `Pubkey`.
    /// * `admin_profile_pda` - The `Pubkey` of the `AdminProfile` **PDA** this user profile is linked to.
    /// * `amount` - The amount of lamports to deposit.
    pub async fn prepare_user_deposit(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
        amount: u64,
    ) -> Result<Transaction, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), admin_profile_pda.as_ref()],
            &w3b2_solana_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::UserDeposit {
                authority,
                user_profile: user_pda,
                admin_profile: admin_profile_pda,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::UserDeposit { amount }.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares a `user_withdraw` transaction.
    ///
    /// * `authority` - The user's wallet `Pubkey`.
    /// * `admin_profile_pda` - The `Pubkey` of the `AdminProfile` **PDA** this user profile is linked to.
    /// * `amount` - The amount of lamports to withdraw.
    /// * `destination` - The `Pubkey` of the wallet to receive the funds.
    pub async fn prepare_user_withdraw(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
        amount: u64,
        destination: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), admin_profile_pda.as_ref()],
            &w3b2_solana_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::UserWithdraw {
                authority,
                user_profile: user_pda,
                admin_profile: admin_profile_pda,
                destination,
            }
            .to_account_metas(None),
            data: instruction::UserWithdraw { amount }.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares a `user_close_profile` transaction.
    ///
    /// * `authority` - The user's wallet `Pubkey`.
    /// * `admin_profile_pda` - The `Pubkey` of the `AdminProfile` **PDA** this user profile is linked to.
    pub async fn prepare_user_close_profile(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), admin_profile_pda.as_ref()],
            &w3b2_solana_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::UserCloseProfile {
                authority,
                user_profile: user_pda,
                admin_profile: admin_profile_pda,
            }
            .to_account_metas(None),
            data: instruction::UserCloseProfile {}.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    // --- Operational Transaction Preparations ---

    /// Prepares a `user_dispatch_command` transaction. This method now requires
    /// a pre-signed message from the oracle. The final transaction will contain
    /// two instructions: the `Ed25519` signature verification, followed by the
    /// actual `user_dispatch_command`.
    ///
    /// * `authority` - The user's wallet `Pubkey`.
    /// * `target_admin_pda` - The `Pubkey` of the target `AdminProfile` **PDA**.
    /// * `command_id` - The `u16` identifier for the command.
    /// * `price` - The price of the command, as signed by the oracle.
    /// * `timestamp` - The timestamp from the oracle's signature.
    /// * `payload` - An opaque byte array for application-specific data.
    /// * `oracle_pubkey` - The public key of the oracle that signed the message.
    /// * `oracle_signature` - The 64-byte Ed25519 signature from the oracle.
    pub async fn prepare_user_dispatch_command(
        &self,
        authority: Pubkey,
        target_admin_pda: Pubkey,
        command_id: u16,
        price: u64,
        timestamp: i64,
        payload: Vec<u8>,
        oracle_pubkey: Pubkey,
        oracle_signature: [u8; 64],
    ) -> Result<Transaction, ClientError> {
        // 1. Reconstruct the message that the oracle signed.
        let message = [
            command_id.to_le_bytes().as_ref(),
            price.to_le_bytes().as_ref(),
            timestamp.to_le_bytes().as_ref(),
        ]
        .concat();

        // 2. Create the Ed25519 signature verification instruction.
        let ed25519_ix = new_ed25519_instruction_with_signature(
            &message,
            &oracle_signature,
            &oracle_pubkey.to_bytes(),
        );

        // 3. Create the main `user_dispatch_command` instruction.
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), target_admin_pda.as_ref()],
            &w3b2_solana_program::ID,
        );

        let dispatch_ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::UserDispatchCommand {
                authority,
                user_profile: user_pda,
                admin_profile: target_admin_pda,
                instructions: sysvar::instructions::id(),
            }
            .to_account_metas(None),
            data: instruction::UserDispatchCommand {
                command_id,
                price,
                timestamp,
                payload,
            }
            .data(),
        };

        // 4. Create a transaction containing both instructions in the correct order.
        self.create_transaction_with_instructions(&authority, vec![ed25519_ix, dispatch_ix])
            .await
    }

    /// Prepares a `log_action` transaction. This instruction requires both the
    /// `UserProfile` and `AdminProfile` PDAs to be passed in the accounts list
    /// to ensure the action is logged within a valid, existing relationship.
    ///
    /// * `authority` - The `Pubkey` of the signer (can be user or admin wallet).
    /// * `user_profile_pda` - The `Pubkey` of the `UserProfile` **PDA**.
    /// * `admin_profile_pda` - The `Pubkey` of the `AdminProfile` **PDA**.
    /// * `session_id` - A `u64` identifier to correlate actions.
    /// * `action_code` - A `u16` code for the specific action.
    pub async fn prepare_log_action(
        &self,
        authority: Pubkey,
        user_profile_pda: Pubkey,
        admin_profile_pda: Pubkey,
        session_id: u64,
        action_code: u16,
    ) -> Result<Transaction, ClientError> {
        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::LogAction {
                authority,
                user_profile: user_profile_pda,
                admin_profile: admin_profile_pda,
            }
            .to_account_metas(None),
            data: instruction::LogAction {
                session_id,
                action_code,
            }
            .data(),
        };

        self.create_transaction(&authority, ix).await
    }
}
