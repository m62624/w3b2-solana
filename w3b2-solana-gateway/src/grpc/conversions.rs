use crate::grpc::proto::w3b2::protocol::gateway;
use w3b2_solana_connector::events as ConnectorEvents;

impl From<ConnectorEvents::EventSource> for gateway::EventSource {
    fn from(source: ConnectorEvents::EventSource) -> Self {
        match source {
            ConnectorEvents::EventSource::Live => gateway::EventSource::Live,
            ConnectorEvents::EventSource::Catchup => gateway::EventSource::Catchup,
        }
    }
}

impl From<ConnectorEvents::BridgeEvent> for gateway::EventStreamItem {
    fn from(event: ConnectorEvents::BridgeEvent) -> Self {
        let source = gateway::EventSource::from(event.source);
        let event_data = event.data;

        let bridge_event_oneof = match event_data {
            ConnectorEvents::BridgeEventData::AdminProfileRegistered(e) => {
                Some(gateway::bridge_event::Event::AdminProfileRegistered(
                    gateway::AdminProfileRegistered {
                        admin_pda: e.admin_pda.to_string(),
                        authority: e.authority.to_string(),
                        communication_pubkey: e.communication_pubkey.to_string(),
                        ts: e.ts,
                    },
                ))
            }
            ConnectorEvents::BridgeEventData::AdminConfigUpdated(e) => Some(
                gateway::bridge_event::Event::AdminConfigUpdated(gateway::AdminConfigUpdated {
                    authority: e.authority.to_string(),
                    admin_pda: e.admin_pda.to_string(),
                    new_oracle_authority: e.new_oracle_authority.to_string(),
                    new_timestamp_validity: e.new_timestamp_validity,
                    new_communication_pubkey: e.new_communication_pubkey.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEventData::AdminFundsWithdrawn(e) => Some(
                gateway::bridge_event::Event::AdminFundsWithdrawn(gateway::AdminFundsWithdrawn {
                    authority: e.authority.to_string(),
                    admin_pda: e.admin_pda.to_string(),
                    amount: e.amount,
                    destination: e.destination.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEventData::AdminProfileClosed(e) => Some(
                gateway::bridge_event::Event::AdminProfileClosed(gateway::AdminProfileClosed {
                    authority: e.authority.to_string(),
                    admin_pda: e.admin_pda.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEventData::AdminCommandDispatched(e) => {
                Some(gateway::bridge_event::Event::AdminCommandDispatched(
                    gateway::AdminCommandDispatched {
                        sender: e.sender.to_string(),
                        sender_admin_pda: e.sender_admin_pda.to_string(),
                        target_user_pda: e.target_user_pda.to_string(),
                        command_id: e.command_id as u32,
                        payload: e.payload,
                        ts: e.ts,
                    },
                ))
            }
            ConnectorEvents::BridgeEventData::UserProfileCreated(e) => Some(
                gateway::bridge_event::Event::UserProfileCreated(gateway::UserProfileCreated {
                    authority: e.authority.to_string(),
                    user_pda: e.user_pda.to_string(),
                    target_admin_pda: e.target_admin_pda.to_string(),
                    communication_pubkey: e.communication_pubkey.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEventData::UserCommKeyUpdated(e) => Some(
                gateway::bridge_event::Event::UserCommKeyUpdated(gateway::UserCommKeyUpdated {
                    authority: e.authority.to_string(),
                    user_profile_pda: e.user_profile_pda.to_string(),
                    new_comm_pubkey: e.new_comm_pubkey.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEventData::UserFundsDeposited(e) => Some(
                gateway::bridge_event::Event::UserFundsDeposited(gateway::UserFundsDeposited {
                    authority: e.authority.to_string(),
                    user_profile_pda: e.user_profile_pda.to_string(),
                    amount: e.amount,
                    new_deposit_balance: e.new_deposit_balance,
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEventData::UserFundsWithdrawn(e) => Some(
                gateway::bridge_event::Event::UserFundsWithdrawn(gateway::UserFundsWithdrawn {
                    authority: e.authority.to_string(),
                    user_profile_pda: e.user_profile_pda.to_string(),
                    amount: e.amount,
                    destination: e.destination.to_string(),
                    new_deposit_balance: e.new_deposit_balance,
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEventData::UserProfileClosed(e) => Some(
                gateway::bridge_event::Event::UserProfileClosed(gateway::UserProfileClosed {
                    authority: e.authority.to_string(),
                    user_pda: e.user_pda.to_string(),
                    admin_pda: e.admin_pda.to_string(),
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEventData::UserCommandDispatched(e) => {
                Some(gateway::bridge_event::Event::UserCommandDispatched(
                    gateway::UserCommandDispatched {
                        sender: e.sender.to_string(),
                        sender_user_pda: e.sender_user_pda.to_string(),
                        target_admin_pda: e.target_admin_pda.to_string(),
                        command_id: e.command_id as u32,
                        price_paid: e.price_paid,
                        payload: e.payload,
                        ts: e.ts,
                    },
                ))
            }
            ConnectorEvents::BridgeEventData::OffChainActionLogged(e) => Some(
                gateway::bridge_event::Event::OffChainActionLogged(gateway::OffChainActionLogged {
                    actor: e.actor.to_string(),
                    user_profile_pda: e.user_profile_pda.to_string(),
                    admin_profile_pda: e.admin_profile_pda.to_string(),
                    session_id: e.session_id,
                    action_code: e.action_code as u32,
                    ts: e.ts,
                }),
            ),
            ConnectorEvents::BridgeEventData::Unknown => None,
        };

        let bridge_event = gateway::BridgeEvent {
            event: bridge_event_oneof,
        };

        Self {
            source: source as i32,
            event: Some(bridge_event),
        }
    }
}
