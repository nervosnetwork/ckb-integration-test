use super::SharedState;
use ckb_network::{
    compress::{compress, decompress},
    SupportProtocols,
};
use p2p::{
    builder::MetaBuilder as P2PMetaBuilder,
    context::{ProtocolContext, ProtocolContextMutRef},
    service::{ProtocolHandle as P2PProtocolHandle, ProtocolMeta as P2PProtocolMeta},
    traits::ServiceProtocol as P2PServiceProtocol,
};
use std::sync::{Arc, RwLock};

/// Simple protocol handler which implements tentacle's
/// [`P2PServiceProtocol`](https://github.com/nervosnetwork/tentacle/blob/master/tentacle/src/traits.rs#L57-L77)
pub struct SimpleProtocolHandler {
    shared: Arc<RwLock<SharedState>>,
    protocol: SupportProtocols,
}

impl SimpleProtocolHandler {
    pub fn new(shared: Arc<RwLock<SharedState>>, protocol: SupportProtocols) -> Self {
        Self { shared , protocol}
    }

    pub fn build(self, be_compressed: bool) -> P2PProtocolMeta {
        let meta_builder: P2PMetaBuilder = self.protocol .clone().into();
        if  be_compressed {
            meta_builder
                .before_send(compress)
                .before_receive(|| Some(Box::new(decompress)))
                .service_handle(move || P2PProtocolHandle::Callback(Box::new(self)))
                .build()
        } else {
            meta_builder
                .service_handle(move || P2PProtocolHandle::Callback(Box::new(self)))
                .build()
        }
    }
}

impl P2PServiceProtocol for SimpleProtocolHandler {
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