use super::SharedState;
use ckb_network::SupportProtocols;
use p2p::{
    builder::MetaBuilder as P2PMetaBuilder,
    context::{ProtocolContext, ProtocolContextMutRef},
    service::{ProtocolHandle as P2PProtocolHandle, ProtocolMeta as P2PProtocolMeta},
    traits::ServiceProtocol as P2PServiceProtocol,
};
use std::sync::{Arc, RwLock};

pub struct DiscoveryProtocolHandler {
    shared: Arc<RwLock<SharedState>>,
}

impl DiscoveryProtocolHandler {
    pub fn new(shared: Arc<RwLock<SharedState>>) -> Self {
        Self { shared }
    }

    pub fn build(self) -> P2PProtocolMeta {
        let meta_builder: P2PMetaBuilder = SupportProtocols::Discovery.into();
        meta_builder
            // .before_send(compress)
            // .before_receive(|| Some(Box::new(decompress)))
            .service_handle(move || P2PProtocolHandle::Callback(Box::new(self)))
            .build()
    }
}

impl P2PServiceProtocol for DiscoveryProtocolHandler {
    fn init(&mut self, _context: &mut ProtocolContext) {}

    fn connected(&mut self, context: ProtocolContextMutRef, _protocol_version: &str) {
        if let Ok(mut shared) = self.shared.write() {
            shared.add_protocol(context.session.id, context.proto_id);
        }
    }

    fn disconnected(&mut self, context: ProtocolContextMutRef) {
        if let Ok(mut shared) = self.shared.write() {
            shared.remove_protocol(&context.session.id, &context.proto_id());
        }
    }
}
