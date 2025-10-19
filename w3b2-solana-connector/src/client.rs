//! # Legacy Transaction Builder
//!
//! This module provides the [`TransactionBuilder`], a utility for creating unsigned
//! Solana transaction messages for the `w3b2-solana-program`.
//!
//! ## Use Case
//!
//! This builder is a helper for **off-chain Rust services** (e.g., an oracle,
//! a custom admin tool) that need to construct instructions programmatically in Rust.
//! It simplifies the process of building valid `Message` objects that can then be
//! signed and sent to the blockchain.
//!
//! It is **not** the primary or recommended way for typical clients to interact with
//! the on-chain program. Standard clients (web, mobile) should use the program's IDL
//! with mainstream libraries like `@coral-xyz/anchor` (TypeScript) or `anchorpy` (Python).
//! This builder is also **not** used by the gRPC gateway.
//!
//! ## Features
//!
//! - **Async API**: All methods are `async`.
//! - **RPC Abstraction**: Uses a generic [`AsyncRpcClient`] trait, making it compatible
//!   with both the live `RpcClient` and the `BanksClient` for integration tests.
//! - **Comprehensive Coverage**: Provides a `prepare_` method for every instruction.
//! - **Security**: Handles the complexity of instruction creation, such as deriving
//!   PDAs and composing multi-instruction transactions (like the `Ed25519`
//!   verification required for `user_dispatch_command`).

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

pub use crate::dispatcher::UserDispatchCommandArgs;

/// A trait abstracting over the asynchronous RPC client functionality.
///
/// This allows the [`TransactionBuilder`] to be generic over the RPC client,
/// making it easy to use with both the live `RpcClient` and the `BanksClient` for integration tests.
#[async_trait]
pub trait AsyncRpcClient: Send + Sync {
    /// Fetches the latest blockhash from the RPC endpoint.
    async fn get_latest_blockhash(&self) -> Result<Hash, ClientError>;
    /// Sends and confirms a transaction, waiting for it to be finalized.
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
impl<C> TransactionBuilder<C>
where
    C: AsyncRpcClient + ?Sized,
{
    /// Submits a signed transaction and waits for confirmation.
    pub async fn submit_transaction(&self, tx: &Transaction) -> Result<Signature, ClientError> {
        self.rpc_client.send_and_confirm_transaction(tx).await
    }

    /// A private helper to create a message from a vector of instructions.
    ///
    /// This function encapsulates the boilerplate of creating a new message
    /// with a specified fee payer.
    fn create_message_with_instructions(payer: &Pubkey, instructions: Vec<Instruction>) -> Vec<u8> {
        // Using `Message::new` is more robust as it correctly deduces the fee payer
        // from the first account in the first instruction that is a signer.
        // We ensure the payer is the first signer account for clarity.
        let msg = solana_sdk::message::Message::new(&instructions, Some(payer));
        bincode::serde::encode_to_vec(&msg, bincode::config::standard()).unwrap()
    }
}

/// A builder for preparing unsigned on-chain transactions.
///
/// This struct provides `prepare_` methods to construct unsigned transactions for
/// every instruction in the `w3b2-solana-program`. The calling service is
/// responsible for signing and submitting the resulting transaction.
#[derive(Clone)]
pub struct TransactionBuilder<C: AsyncRpcClient + ?Sized> {
    /// A shared, thread-safe reference to a Solana JSON RPC client.
    rpc_client: Arc<C>,
}

impl<C: AsyncRpcClient + ?Sized> TransactionBuilder<C> {
    /// Creates a new `TransactionBuilder`.
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - A shared client that implements [`AsyncRpcClient`] (e.g., `Arc<RpcClient>`).
    pub fn new(rpc_client: Arc<C>) -> Self {
        Self { rpc_client }
    }

    // --- Admin Transaction Preparations ---

    /// Prepares an `admin_register_profile` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The public key of the admin's wallet that will sign the transaction.
    /// * `communication_pubkey` - The public key for secure off-chain communication.
    pub fn prepare_admin_register_profile(
        &self,
        authority: Pubkey,
        communication_pubkey: Pubkey,
    ) -> Vec<u8> {
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

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    /// Prepares an `admin_ban_user` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The public key of the admin's wallet.
    /// * `target_user_profile_pda` - The PDA of the `UserProfile` to be banned.
    pub fn prepare_admin_ban_user(
        &self,
        authority: Pubkey,
        target_user_profile_pda: Pubkey,
    ) -> Vec<u8> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_solana_program::ID);

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::AdminBanUser {
                authority,
                admin_profile: admin_pda,
                user_profile: target_user_profile_pda,
            }
            .to_account_metas(None),
            data: instruction::AdminBanUser {}.data(),
        };

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    /// Prepares an `admin_unban_user` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The public key of the admin's wallet.
    /// * `target_user_profile_pda` - The PDA of the `UserProfile` to be unbanned.
    pub fn prepare_admin_unban_user(
        &self,
        authority: Pubkey,
        target_user_profile_pda: Pubkey,
    ) -> Vec<u8> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_solana_program::ID);

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::AdminUnbanUser {
                authority,
                admin_profile: admin_pda,
                user_profile: target_user_profile_pda,
            }
            .to_account_metas(None),
            data: instruction::AdminUnbanUser {}.data(),
        };

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    /// Prepares an `admin_set_config` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The public key of the admin's wallet.
    /// * `new_oracle_authority` - An optional new `Pubkey` for the oracle.
    /// * `new_timestamp_validity` - An optional new duration in seconds for signature validity.
    /// * `new_communication_pubkey` - An optional new `Pubkey` for off-chain communication.
    /// * `new_unban_fee` - An optional new fee in lamports for unban requests.
    pub fn prepare_admin_set_config(
        &self,
        authority: Pubkey,
        new_oracle_authority: Option<Pubkey>,
        new_timestamp_validity: Option<i64>,
        new_communication_pubkey: Option<Pubkey>,
        new_unban_fee: Option<u64>,
    ) -> Vec<u8> {
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
                new_unban_fee,
            }
            .data(),
        };

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    /// Prepares an `admin_withdraw` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The public key of the admin's wallet.
    /// * `amount` - The amount of lamports to withdraw from the `AdminProfile` balance.
    /// * `destination` - The public key of the account to receive the funds.
    pub fn prepare_admin_withdraw(
        &self,
        authority: Pubkey,
        amount: u64,
        destination: Pubkey,
    ) -> Vec<u8> {
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

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    /// Prepares an `admin_close_profile` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The public key of the admin's wallet.
    pub fn prepare_admin_close_profile(&self, authority: Pubkey) -> Vec<u8> {
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

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    /// Prepares an `admin_dispatch_command` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The public key of the admin's wallet.
    /// * `target_user_profile_pda` - The PDA of the target `UserProfile`.
    /// * `command_id` - A `u64` identifier for the command.
    /// * `payload` - An opaque byte array for application-specific data.
    pub fn prepare_admin_dispatch_command(
        &self,
        authority: Pubkey,
        target_user_profile_pda: Pubkey,
        command_id: u64,
        payload: Vec<u8>,
    ) -> Vec<u8> {
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

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    // --- User Transaction Preparations ---

    /// Prepares a `user_create_profile` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The user's wallet `Pubkey` that will sign and own the profile.
    /// * `target_admin_pda` - The `Pubkey` of the `AdminProfile` PDA to link to.
    /// * `communication_pubkey` - The user's public key for off-chain communication.
    pub fn prepare_user_create_profile(
        &self,
        authority: Pubkey,
        target_admin_pda: Pubkey,
        communication_pubkey: Pubkey,
    ) -> Vec<u8> {
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

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    /// Prepares a `user_update_comm_key` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The user's wallet `Pubkey`.
    /// * `admin_profile_pda` - The `Pubkey` of the `AdminProfile` PDA this user profile is linked to.
    /// * `new_key` - The new communication key to set.
    pub fn prepare_user_update_comm_key(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
        new_key: Pubkey,
    ) -> Vec<u8> {
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

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    /// Prepares a `user_deposit` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The user's wallet `Pubkey`.
    /// * `admin_profile_pda` - The `Pubkey` of the `AdminProfile` PDA this user profile is linked to.
    /// * `amount` - The amount of lamports to deposit.
    pub fn prepare_user_deposit(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
        amount: u64,
    ) -> Vec<u8> {
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

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    /// Prepares a `user_withdraw` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The user's wallet `Pubkey`.
    /// * `admin_profile_pda` - The `Pubkey` of the `AdminProfile` PDA this user profile is linked to.
    /// * `amount` - The amount of lamports to withdraw.
    /// * `destination` - The `Pubkey` of the wallet to receive the funds.
    pub fn prepare_user_withdraw(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
        amount: u64,
        destination: Pubkey,
    ) -> Vec<u8> {
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

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    /// Prepares a `user_close_profile` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The user's wallet `Pubkey`.
    /// * `admin_profile_pda` - The `Pubkey` of the `AdminProfile` PDA this user profile is linked to.
    pub fn prepare_user_close_profile(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
    ) -> Vec<u8> {
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

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    // --- Operational Transaction Preparations ---

    /// Prepares a `user_dispatch_command` transaction.
    ///
    /// This method creates a transaction containing two instructions in the correct order:
    /// 1.  An `Ed25519` signature verification instruction.
    /// 2.  The actual `user_dispatch_command` instruction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The user's wallet `Pubkey` that will sign the transaction.
    /// * `target_admin_pda` - The `Pubkey` of the target `AdminProfile` PDA.
    /// * `args` - A [`UserDispatchCommandArgs`] struct containing all oracle-signed parameters.
    pub fn prepare_user_dispatch_command(
        &self,
        authority: Pubkey,
        target_admin_pda: Pubkey,
        args: UserDispatchCommandArgs,
    ) -> Vec<u8> {
        // 1. Reconstruct the message that the oracle signed.
        let message = [
            args.command_id.to_le_bytes().as_ref(),
            args.price.to_le_bytes().as_ref(),
            args.timestamp.to_le_bytes().as_ref(),
        ]
        .concat();

        // 2. Create the Ed25519 signature verification instruction.
        let ed25519_ix = new_ed25519_instruction_with_signature(
            &message,
            &args.oracle_signature,
            &args.oracle_pubkey.to_bytes(),
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
                command_id: args.command_id,
                price: args.price,
                timestamp: args.timestamp,
                payload: args.payload,
            }
            .data(),
        };

        // 4. Create a transaction containing both instructions in the correct order.
        TransactionBuilder::<C>::create_message_with_instructions(
            &authority,
            vec![ed25519_ix, dispatch_ix],
        )
    }

    /// Prepares a `user_request_unban` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The user's wallet `Pubkey`.
    /// * `admin_profile_pda` - The `Pubkey` of the `AdminProfile` PDA this user profile is linked to.
    pub fn prepare_user_request_unban(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
    ) -> Vec<u8> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), admin_profile_pda.as_ref()],
            &w3b2_solana_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_solana_program::ID,
            accounts: accounts::UserRequestUnban {
                authority,
                user_profile: user_pda,
                admin_profile: admin_profile_pda,
            }
            .to_account_metas(None),
            data: instruction::UserRequestUnban {}.data(),
        };

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }

    /// Prepares a `log_action` transaction.
    ///
    /// This instruction requires both the `UserProfile` and `AdminProfile` PDAs to be
    /// passed in the accounts list to ensure the action is logged within a valid,
    /// existing relationship.
    ///
    /// # Arguments
    ///
    /// * `authority` - The `Pubkey` of the signer (can be user or admin wallet).
    /// * `user_profile_pda` - The `Pubkey` of the `UserProfile` PDA.
    /// * `admin_profile_pda` - The `Pubkey` of the `AdminProfile` PDA.
    /// * `session_id` - A `u64` identifier to correlate actions.
    /// * `action_code` - A `u16` code for the specific action.
    pub fn prepare_log_action(
        &self,
        authority: Pubkey,
        user_profile_pda: Pubkey,
        admin_profile_pda: Pubkey,
        session_id: u64,
        action_code: u16,
    ) -> Vec<u8> {
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

        TransactionBuilder::<C>::create_message_with_instructions(&authority, vec![ix])
    }
}
