//! Defines a generic RPC client trait to abstract over different client implementations.

use async_trait::async_trait;
use solana_client::{client_error::ClientError, nonblocking::rpc_client::RpcClient};
use solana_program_test::BanksClient;
use solana_sdk::{
    hash::Hash, signature::Signature, transaction::Transaction, transport::TransportError,
};

/// A generic trait for a Solana RPC client.
///
/// This trait abstracts over the specific client implementation, allowing for the use of
/// both a real `RpcClient` for live environments and a `BanksClient` for testing with
/// `solana-program-test`.
#[async_trait]
pub trait GenericRpcClient: Send + Sync {
    /// Gets the latest blockhash.
    async fn get_latest_blockhash(&self) -> Result<Hash, ClientError>;

    /// Sends and confirms a transaction.
    async fn send_and_confirm_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Signature, ClientError>;
}

#[async_trait]
impl GenericRpcClient for RpcClient {
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

#[async_trait]
impl GenericRpcClient for BanksClient {
    async fn get_latest_blockhash(&self) -> Result<Hash, ClientError> {
        let mut client = self.clone();
        client
            .get_latest_blockhash()
            .await
            .map_err(|e| ClientError::from(TransportError::Custom(e.to_string())))
    }

    async fn send_and_confirm_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Signature, ClientError> {
        let mut client = self.clone();
        client
            .process_transaction(transaction.clone())
            .await
            .unwrap();
        Ok(transaction.signatures[0])
    }
}
