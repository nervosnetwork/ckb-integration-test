use super::Connector;
use crate::preclude::*;
use ckb_network::SupportProtocols;
use ckb_types::{
    core::{Cycle, TransactionView},
    packed,
    prelude::*,
};

impl Connector {
    pub fn send_relay_transaction(
        &self,
        node: &Node,
        transaction: &TransactionView,
        cycles: Cycle,
    ) -> Result<(), String> {
        let protocol_id = SupportProtocols::Relay.protocol_id();
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
        self.send(&node, protocol_id, relay_message.as_bytes())?;
        Ok(())
    }

    pub fn send_relay_v2_transaction(
        &self,
        node: &Node,
        transaction: &TransactionView,
        cycles: Cycle,
    ) -> Result<(), String> {
        let protocol_id = SupportProtocols::RelayV2.protocol_id();
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
        self.send(&node, protocol_id, relay_message.as_bytes())?;
        Ok(())
    }
}
