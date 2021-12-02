//! # Identify Protocol
//!
//! ## Note
//!
//! * Identify protocol is a short-running protocol. Two nodes exchange their node identify
//! information and close the protocol right away.

use super::SharedState;
use ckb_network::{
    compress::{compress, decompress},
    SupportProtocols,
};
use ckb_types::{packed, prelude::*};
use p2p::{
    builder::MetaBuilder as P2PMetaBuilder,
    bytes::Bytes,
    context::{ProtocolContext, ProtocolContextMutRef},
    service::{ProtocolHandle as P2PProtocolHandle, ProtocolMeta as P2PProtocolMeta},
    traits::ServiceProtocol as P2PServiceProtocol,
};
use std::sync::{Arc, RwLock};

/// Identify protocol handler which implements tentacle's
/// [`P2PServiceProtocol`](https://github.com/nervosnetwork/tentacle/blob/master/tentacle/src/traits.rs#L57-L77)
pub struct IdentifyProtocolHandler {
    shared: Arc<RwLock<SharedState>>,
    network_identifier: String,
    client_version: String,
}

impl IdentifyProtocolHandler {
    pub fn new(
        shared: Arc<RwLock<SharedState>>,
        network_identifier: String,
        client_version: String,
    ) -> Self {
        Self {
            shared,
            network_identifier,
            client_version,
        }
    }

    pub fn build(self) -> P2PProtocolMeta {
        let meta_builder: P2PMetaBuilder = SupportProtocols::Identify.into();
        meta_builder
            .before_send(compress)
            .before_receive(|| Some(Box::new(decompress)))
            .service_handle(move || P2PProtocolHandle::Callback(Box::new(self)))
            .build()
    }
}

impl P2PServiceProtocol for IdentifyProtocolHandler {
    fn init(&mut self, _context: &mut ProtocolContext) {}

    fn connected(&mut self, context: ProtocolContextMutRef, _protocol_version: &str) {
        if let Ok(mut shared) = self.shared.write() {
            shared.add_protocol(context.session.id, context.proto_id);
        }

        let identify_self_defined_payload = packed::Identify::new_builder()
            .name(self.network_identifier.as_str().pack())
            .client_version(self.client_version.as_str().pack())
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
            // .listen_addrs( vec![])
            .observed_addr({
                let byte_vec = context
                    .session
                    .address
                    .to_vec()
                    .into_iter()
                    .map(Into::into)
                    .collect();
                let bytes = packed::Bytes::new_builder().set(byte_vec).build();
                packed::Address::new_builder().bytes(bytes).build()
            })
            .build();
        context
            .quick_send_message(identify_message.as_bytes())
            .unwrap();
    }

    fn disconnected(&mut self, context: ProtocolContextMutRef) {
        if let Ok(mut shared) = self.shared.write() {
            shared.remove_protocol(&context.session.id, &context.proto_id());
        }
    }

    fn received(&mut self, context: ProtocolContextMutRef, _data: Bytes) {
        crate::info!(
            "IdentifyProtocol received from session: {:?}",
            context.session
        );
    }
}
