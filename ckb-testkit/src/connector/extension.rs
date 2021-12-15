/// Util functions attached to `Connector`.
///
use super::Connector;
use super::SupportProtocols;
use crate::Node;
use ckb_types::{
    core::{Cycle, TransactionView},
    packed,
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
        let relay_tx = packed::RelayTransaction::new_builder()
            .transaction(transaction.data())
            .cycles(cycles.pack())
            .build();
        let relay_tx_vec = packed::RelayTransactionVec::new_builder()
            .push(relay_tx)
            .build();
        let relay_txs = packed::RelayTransactions::new_builder()
            .transactions(relay_tx_vec)
            .build();
        let relay_message = packed::RelayMessage::new_builder().set(relay_txs).build();
        self.send(&node, SupportProtocols::Relay, relay_message.as_bytes())?;
        Ok(())
    }

    pub fn send_relay_v2_transaction(
        &self,
        node: &Node,
        transaction: &TransactionView,
        cycles: Cycle,
    ) -> Result<(), String> {
        let relay_tx = packed::RelayTransaction::new_builder()
            .transaction(transaction.data())
            .cycles(cycles.pack())
            .build();
        let relay_tx_vec = packed::RelayTransactionVec::new_builder()
            .push(relay_tx)
            .build();
        let relay_txs = packed::RelayTransactions::new_builder()
            .transactions(relay_tx_vec)
            .build();
        let relay_message = packed::RelayMessage::new_builder().set(relay_txs).build();
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
        let identify_self_defined_payload = packed::Identify::new_builder()
            .name(network_identifier.pack())
            .client_version(client_version.pack())
            .flag({
                // https://github.com/nervosnetwork/ckb/blob/3f89ae6dd2e0fd86b899b0c37dbe11864dc16544/network/src/protocols/identify/mod.rs#L604
                const FLAG_FULL_NODE: u64 = 1;
                FLAG_FULL_NODE.pack()
            })
            .build();
        let identify_message = packed::IdentifyMessage::new_builder()
            .identify({
                packed::Bytes::new_builder()
                    .set(
                        identify_self_defined_payload
                            .as_bytes()
                            .to_vec()
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                    )
                    .build()
            })
            .listen_addrs({
                let to_vec = listening_addresses
                    .into_iter()
                    .map(|addr| packed::Address::from_slice(&addr.to_vec()).unwrap())
                    .collect::<Vec<_>>();
                packed::AddressVec::new_builder().set(to_vec).build()
            })
            .observed_addr({
                let byte_vec = observed_address
                    .to_vec()
                    .into_iter()
                    .map(Into::into)
                    .collect();
                let bytes = packed::Bytes::new_builder().set(byte_vec).build();
                packed::Address::new_builder().bytes(bytes).build()
            })
            .build();
        self.send(
            node,
            SupportProtocols::Identify,
            identify_message.as_bytes(),
        )?;
        Ok(())
    }
}
