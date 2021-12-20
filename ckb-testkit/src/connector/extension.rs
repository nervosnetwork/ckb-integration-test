/// Util functions attached to `Connector`.
///
use super::{
    message::{
        build_discovery_get_nodes, build_discovery_nodes, build_identify_message,
        build_relay_transaction,
    },
    Connector, SupportProtocols,
};
use crate::Node;
use ckb_types::{
    core::{Cycle, TransactionView},
    prelude::*,
};
use p2p::multiaddr::Multiaddr;

impl Connector {
    pub fn send_relay_transaction(
        &self,
        node: &Node,
        transaction: &TransactionView,
        cycles: Cycle,
    ) -> Result<(), String> {
        let relay_message = build_relay_transaction(transaction, cycles);
        self.send(&node, SupportProtocols::Relay, relay_message.as_bytes())?;
        Ok(())
    }

    pub fn send_relay_v2_transaction(
        &self,
        node: &Node,
        transaction: &TransactionView,
        cycles: Cycle,
    ) -> Result<(), String> {
        let relay_message = build_relay_transaction(transaction, cycles);
        self.send(&node, SupportProtocols::RelayV2, relay_message.as_bytes())?;
        Ok(())
    }

    pub fn send_identify_message(
        &self,
        node: &Node,
        network_identifier: &str,
        client_version: &str,
        listening_addresses: Vec<Multiaddr>,
        observed_address: Multiaddr,
    ) -> Result<(), String> {
        let identify_message = build_identify_message(
            network_identifier,
            client_version,
            listening_addresses,
            observed_address,
        );
        self.send(
            node,
            SupportProtocols::Identify,
            identify_message.as_bytes(),
        )?;
        Ok(())
    }

    pub fn send_discovery_get_nodes(
        &self,
        node: &Node,
        listening_port: Option<u16>,
        max_nodes: u32,
        self_defined_flag: u32,
    ) -> Result<(), String> {
        let discovery_message =
            build_discovery_get_nodes(listening_port, max_nodes, self_defined_flag);
        self.send(
            node,
            SupportProtocols::Discovery,
            discovery_message.as_bytes(),
        )?;
        Ok(())
    }

    pub fn send_discovery_nodes(
        &self,
        node: &Node,
        active_push: bool,
        addresses: Vec<Multiaddr>,
    ) -> Result<(), String> {
        let discovery_message = build_discovery_nodes(active_push, addresses);
        self.send(
            node,
            SupportProtocols::Discovery,
            discovery_message.as_bytes(),
        )?;
        Ok(())
    }
}
