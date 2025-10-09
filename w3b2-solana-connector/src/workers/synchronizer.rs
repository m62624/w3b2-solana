use crate::{
    config::ConnectorConfig,
    dispatcher::DispatcherHandle,
    storage::Storage,
    workers::{catchup::CatchupWorker, live::LiveWorker},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

/// A shared context containing all dependencies required by the workers.
#[derive(Clone)]
pub(crate) struct WorkerContext {
    pub config: Arc<ConnectorConfig>,
    pub storage: Arc<dyn Storage>,
    pub rpc_client: Arc<RpcClient>,
    pub dispatcher: DispatcherHandle,
}

impl WorkerContext {
    fn new(
        config: Arc<ConnectorConfig>,
        rpc_client: Arc<RpcClient>,
        storage: Arc<dyn Storage>,
        dispatcher: DispatcherHandle,
    ) -> Self {
        Self {
            config,
            storage,
            rpc_client,
            dispatcher,
        }
    }
}

/// Orchestrates the `CatchupWorker` and `LiveWorker` to ensure comprehensive
/// and resilient synchronization with the Solana blockchain.
///
/// The Synchronizer's primary role is to run both workers concurrently. It acts
/// as a simple container and entry point for the dual-worker synchronization strategy,
/// where the `LiveWorker` handles real-time events and the `CatchupWorker` fills
/// in any historical gaps.
pub struct Synchronizer {
    catchup_worker: CatchupWorker,
    live_worker: LiveWorker,
}

impl Synchronizer {
    /// Creates a new `Synchronizer` instance.
    ///
    /// This constructor initializes the shared `WorkerContext` and uses it to create
    /// instances of `CatchupWorker` and `LiveWorker`. The workers are prepared but
    /// not started until the `run` method is called.
    pub fn new(
        config: Arc<ConnectorConfig>,
        rpc_client: Arc<RpcClient>,
        storage: Arc<dyn Storage>,
        dispatcher: DispatcherHandle,
    ) -> Self {
        let context = WorkerContext::new(config, rpc_client, storage, dispatcher);
        let catchup_worker = CatchupWorker::new(context.clone());
        let live_worker = LiveWorker::new(context);

        Self {
            catchup_worker,
            live_worker,
        }
    }

    /// Runs both the `CatchupWorker` and `LiveWorker` concurrently.
    ///
    /// This is the main execution method for the synchronization process. It uses
    /// `tokio::try_join!` to spawn both workers. The `try_join!` macro ensures that
    /// if either worker returns an error, the other worker is immediately cancelled,
    /// and the error is propagated up. The method will run indefinitely until one
    /// of the workers fails or the parent task is cancelled.
    pub async fn run(self) -> anyhow::Result<()> {
        tracing::info!("Starting synchronizer workers...");

        tokio::try_join!(self.catchup_worker.run(), self.live_worker.run())?;

        Ok(())
    }
}
