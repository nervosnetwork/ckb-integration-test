use ckb_channel::{unbounded, Receiver, Sender};
use p2p::{bytes::Bytes, context::SessionContext, multiaddr::Multiaddr, ProtocolId, SessionId};
use std::collections::HashMap;

/// Shared state between protocol handlers and service handler. As it is used across multiple
/// protocols, it should be wrapped into `Arc<RwLock<SharedState>>`.
pub struct SharedState {
    /// Session manager, #{ session.id => ( session, #{ protocol.id => mailbox } ) }
    session_manager: HashMap<
        SessionId,
        (
            SessionContext,
            HashMap<ProtocolId, (Sender<Bytes>, Receiver<Bytes>)>,
        ),
    >,
}

impl SharedState {
    /// Create a shared state.
    pub fn new() -> Self {
        Self {
            session_manager: HashMap::new(),
        }
    }

    pub fn add_session(&mut self, session_id: SessionId, session: SessionContext) {
        let _ = self
            .session_manager
            .insert(session_id, (session, HashMap::new()));
    }

    pub fn remove_session(&mut self, session_id: &SessionId) -> Option<SessionContext> {
        self.session_manager
            .remove(session_id)
            .map(|(session, _mailbox)| session)
    }

    /// Get session by node address
    pub fn get_session(&self, address: &Multiaddr) -> Option<SessionContext> {
        for (session, _) in self.session_manager.values() {
            if &session.address == address {
                return Some(session.clone());
            }
        }
        None
    }

    pub fn add_protocol(&mut self, session_id: SessionId, protocol_id: ProtocolId) {
        let (_session, mailbox) = self
            .session_manager
            .get_mut(&session_id)
            .expect("session is not found");
        assert!(
            !mailbox.contains_key(&protocol_id),
            "protocol is not opened"
        );
        let (protocol_mailbox_sender, protocol_mailbox_receiver) = unbounded::<Bytes>();
        mailbox.insert(
            protocol_id,
            (protocol_mailbox_sender, protocol_mailbox_receiver),
        );
    }

    pub fn remove_protocol(&mut self, session_id: &SessionId, protocol_id: &ProtocolId) {
        let _ = self
            .session_manager
            .get_mut(session_id)
            .map(|(_session, mailbox)| mailbox.remove(protocol_id));
    }

    pub fn get_protocol_sender(
        &self,
        session_id: &SessionId,
        protocol_id: &ProtocolId,
    ) -> Option<Sender<Bytes>> {
        self.session_manager
            .get(session_id)
            .and_then(|(_, mailbox)| {
                mailbox
                    .get(protocol_id)
                    .map(|(sender, _receiver)| sender.clone())
            })
    }

    pub fn get_protocol_receiver(
        &self,
        session_id: &SessionId,
        protocol_id: &ProtocolId,
    ) -> Option<Receiver<Bytes>> {
        self.session_manager
            .get(session_id)
            .and_then(|(_, mailbox)| {
                mailbox
                    .get(protocol_id)
                    .map(|(_sender, receiver)| receiver.clone())
            })
    }

    pub fn get_opened_protocol_ids(&self, session_id: &SessionId) -> Option<Vec<ProtocolId>> {
        self.session_manager
            .get(session_id)
            .map(|(_session, mailbox)| mailbox.keys().cloned().collect())
    }
}
