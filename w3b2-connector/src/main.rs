use anyhow::Result;
use tokio_stream::StreamExt;
use tonic::{transport::Server, Request, Response, Status};
use w3b2_bridge_program::types::CommandMode;
use w3b2_connector::events::BridgeEvent as Event;
use w3b2_connector::{Storage, SyncConfig, Synchronizer};

pub mod bridge_proto {
    tonic::include_proto!("bridge");
}

use bridge_proto::bridge_service_server::{BridgeService, BridgeServiceServer};
use bridge_proto::{BridgeEvent, Empty};

#[derive(Default)]
pub struct BridgeServer {}

#[tonic::async_trait]
impl BridgeService for BridgeServer {
    type StreamEventsStream = tokio_stream::wrappers::ReceiverStream<Result<BridgeEvent, Status>>;

    async fn stream_events(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::StreamEventsStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(16);

        tokio::spawn(async move {
            let config = SyncConfig::default();
            let storage = Storage::new("./w3b2_db").unwrap();

            let mut event_stream = Synchronizer::builder()
                .with_config(config)
                .with_storage(storage)
                .start()
                .await
                .unwrap();

            while let Some(event) = event_stream.next().await {
                let proto_event = match event {
                    Event::AdminRegistered(e) => BridgeEvent {
                        event: Some(bridge_proto::bridge_event::Event::AdminRegistered(
                            bridge_proto::AdminRegistered {
                                admin: e.admin.to_string(),
                                initial_funding: e.initial_funding,
                                ts: e.ts,
                            },
                        )),
                    },
                    Event::UserRegistered(e) => BridgeEvent {
                        event: Some(bridge_proto::bridge_event::Event::UserRegistered(
                            bridge_proto::UserRegistered {
                                user: e.user.to_string(),
                                initial_balance: e.initial_balance,
                                ts: e.ts,
                            },
                        )),
                    },
                    Event::AdminDeactivated(e) => BridgeEvent {
                        event: Some(bridge_proto::bridge_event::Event::AdminDeactivated(
                            bridge_proto::AdminDeactivated {
                                admin: e.admin.to_string(),
                                ts: e.ts,
                            },
                        )),
                    },
                    Event::UserDeactivated(e) => BridgeEvent {
                        event: Some(bridge_proto::bridge_event::Event::UserDeactivated(
                            bridge_proto::UserDeactivated {
                                user: e.user.to_string(),
                                ts: e.ts,
                            },
                        )),
                    },
                    Event::FundingRequested(e) => BridgeEvent {
                        event: Some(bridge_proto::bridge_event::Event::FundingRequested(
                            bridge_proto::FundingRequested {
                                user_wallet: e.user_wallet.to_string(),
                                target_admin: e.target_admin.to_string(),
                                amount: e.amount,
                                ts: e.ts,
                            },
                        )),
                    },
                    Event::FundingApproved(e) => BridgeEvent {
                        event: Some(bridge_proto::bridge_event::Event::FundingApproved(
                            bridge_proto::FundingApproved {
                                user_wallet: e.user_wallet.to_string(),
                                approved_by: e.approved_by.to_string(),
                                amount: e.amount,
                                ts: e.ts,
                            },
                        )),
                    },
                    Event::CommandEvent(e) => BridgeEvent {
                        event: Some(bridge_proto::bridge_event::Event::CommandEvent(
                            bridge_proto::CommandEvent {
                                sender: e.sender.to_string(),
                                target: e.target.to_string(),
                                command_id: e.command_id,
                                mode: match e.mode {
                                    CommandMode::RequestResponse => {
                                        bridge_proto::CommandMode::RequestResponse as i32
                                    }
                                    CommandMode::OneWay => bridge_proto::CommandMode::OneWay as i32,
                                },
                                payload: e.payload,
                                ts: e.ts,
                            },
                        )),
                    },
                    Event::Unknown => BridgeEvent { event: None },
                };

                if tx.send(Ok(proto_event)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "[::1]:50051".parse().unwrap();
    let bridge_service = BridgeServer::default();

    tracing::info!("gRPC server listening on {}", addr);

    Server::builder()
        .add_service(BridgeServiceServer::new(bridge_service))
        .serve(addr)
        .await?;

    Ok(())
}
