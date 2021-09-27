use crate::preclude::*;
use ckb_app_config::NetworkConfig;
use ckb_async_runtime::new_global_runtime;
use ckb_channel::{unbounded, Receiver, Sender};
use ckb_jsonrpc_types::Consensus;
use ckb_network::{
    bytes::Bytes, extract_peer_id, CKBProtocol, CKBProtocolContext, CKBProtocolHandler,
    DefaultExitHandler, NetworkController, NetworkService, NetworkState, PeerIndex, ProtocolId,
    SupportProtocols,
};
use ckb_stop_handler::StopHandler;
use ckb_testkit::util::{find_available_port, temp_path};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

type MessageTxs = HashMap<PeerIndex, Sender<(ProtocolId, Bytes)>>;
type MessageRxs = HashMap<PeerIndex, Receiver<(ProtocolId, Bytes)>>;

// ## Note of CKB Network Workflow
//
// * `NetworkService` maintains `ProtocolHandler`. Everytime receiving messages
//   or establishing protocol connections, `ProtocolHandler` will be involved.
//
// * `NetConsole` maintains `NetworkController`. It queries and manages
//   `NetworkService` through `NetworkController`.
//
// * `NetConsole` is the only gateway we maintain. We would like to synchronize
//   all the `ProtocolHandler` receiving messages and establishing protocol
//   connections. Therefor we let `ProtocolHandler` send the messages to
//   `NetConsole` through channel (I don't recommende maintain state data inside
//   `ProtocolHandler` as it is not shared between protocols).

pub struct NetConsole {
    /// P2p port
    // p2p_port: u16,

    /// Network data directory
    // working_dir: PathBuf,

    /// The started CKB network protocols
    network_protocols: Vec<SupportProtocols>,

    /// The outside controller of network service. It is using for querying
    /// information inside the network service, managing connections and peers,
    /// broadcasting transactions.
    network_controller: NetworkController,

    /// #{ node_id => peer_index }
    peer_indexs: HashMap<String, PeerIndex>,

    /// Messages streams from `ProtocolHandler`
    message_rxs: MessageRxs,

    _async_runtime_stop: StopHandler<()>,
}

/// NetConsole starts a CKB network service and maintain the corresponding network
/// controller inside. It provides a way to communicate with CKB nodes via
/// CKB network protocols directly. For example, send self-defined messages to
/// CKB nodes.
impl NetConsole {
    // TODO fn new_with_config

    /// Start network service
    pub fn start(
        case_name: &str,
        consensus: &Consensus,
        node_version: &str,
        network_protocols: Vec<SupportProtocols>,
    ) -> Self {
        assert!(!network_protocols.is_empty());
        let working_dir = temp_path(case_name, "net-console");
        let p2p_port = find_available_port();
        let network_identify = {
            let genesis_hash = format!("{:x}", consensus.genesis_hash);
            format!("/{}/{}", consensus.id, &genesis_hash[..8])
        };

        let (message_txs, message_rxs) = batch_unbounded();
        let (async_handle, async_runtime_stop) = new_global_runtime();
        let network_service = {
            let network_state = {
                let p2p_listen = format!("/ip4/127.0.0.1/tcp/{}", p2p_port).parse().unwrap();
                Arc::new(
                    NetworkState::from_config(NetworkConfig {
                        listen_addresses: vec![p2p_listen],
                        path: (&working_dir).into(),
                        max_peers: 128,
                        max_outbound_peers: 128,
                        discovery_local_address: true,
                        ping_interval_secs: 15,
                        ping_timeout_secs: 20,
                        ..Default::default()
                    })
                    .unwrap(),
                )
            };
            let ckb_protocols = network_protocols
                .clone()
                .into_iter()
                .map(|protocol| {
                    CKBProtocol::new_with_support_protocol(
                        protocol,
                        Box::new(ProtocolHandler::new(message_txs.clone())),
                        Arc::clone(&network_state),
                    )
                })
                .collect();
            NetworkService::new(
                network_state,
                ckb_protocols,
                vec![],
                network_identify.to_string(),
                node_version.to_string(),
                DefaultExitHandler::default(),
            )
        };
        let network_controller = network_service.start(&async_handle).unwrap();
        Self {
            // p2p_port,
            // working_dir,
            network_protocols,
            network_controller,
            message_rxs,
            peer_indexs: Default::default(),
            _async_runtime_stop: async_runtime_stop,
        }
    }

    /// Return the internal `NetworkController`
    pub fn network_controller(&self) -> &NetworkController {
        &self.network_controller
    }

    /// Try to establish connection to `node`.
    pub fn connect(&mut self, node: &Node) -> Result<PeerIndex, String> {
        self.network_controller()
            .add_node(node.p2p_address_with_node_id().parse().unwrap());

        // Wait for all protocols connections establishment
        let start_time = Instant::now();
        while start_time.elapsed() <= Duration::from_secs(60) {
            for (peer_index, peer) in self.network_controller().connected_peers() {
                let peer_id = extract_peer_id(&peer.connected_addr).expect("extract_peer_id");
                let node_id = peer_id.to_base58();
                if node_id != node.node_id() {
                    continue;
                }

                let all_protocols_connected = self
                    .network_protocols
                    .iter()
                    .all(|p| peer.protocols.contains_key(&p.protocol_id()));
                if all_protocols_connected {
                    self.peer_indexs.insert(node_id, peer_index);
                    return Ok(peer_index);
                }
            }
            sleep(Duration::from_millis(100));
        }

        Err(format!("timeout to connect to {}", node.node_name()))
    }

    /// Send `data` to peer through `protocol_id`.
    pub fn send(&self, node: &Node, protocol_id: ProtocolId, data: Bytes) -> Result<(), String> {
        let node_id = node.node_id();
        let peer_index = self.peer_indexs.get(node_id).expect("non connected peer");
        self.network_controller()
            .send_message_to(*peer_index, protocol_id, data)
            .map_err(|err| format!("{:?}", err))
    }

    pub fn receive_timeout(
        &self,
        node: &Node,
        timeout: Duration,
    ) -> Result<(ProtocolId, Bytes), String> {
        let node_id = node.node_id();
        let peer_index = self
            .peer_indexs
            .get(node_id)
            .unwrap_or_else(|| panic!("non connected peer {}", node.p2p_address()));
        let message_rx = self
            .message_rxs
            .get(&peer_index)
            .expect("batch_unbounded should alloc enough channels");
        message_rx
            .recv_timeout(timeout)
            .map_err(|err| format!("{:?}", err))
    }
}

// ProtocolHandler is not shared between protocols.
struct ProtocolHandler {
    message_txs: MessageTxs,
}

impl ProtocolHandler {
    pub fn new(message_txs: MessageTxs) -> Self {
        Self { message_txs }
    }
}

// `CKBProtocolHandler` is similar to `p2p::ServiceProtocol`.
impl CKBProtocolHandler for ProtocolHandler {
    fn init(&mut self, _nc: Arc<dyn CKBProtocolContext + Sync>) {}

    fn connected(
        &mut self,
        _nc: Arc<dyn CKBProtocolContext + Sync>,
        _peer_index: PeerIndex,
        _version: &str,
    ) {
    }

    fn disconnected(&mut self, _nc: Arc<dyn CKBProtocolContext + Sync>, _peer_index: PeerIndex) {}

    fn received(
        &mut self,
        nc: Arc<dyn CKBProtocolContext + Sync>,
        peer_index: PeerIndex,
        data: Bytes,
    ) {
        let message_tx = self
            .message_txs
            .get(&peer_index)
            .expect("batch_unbounded should alloc enough channels");
        if let Err(err) = message_tx.send((nc.protocol_id(), data)) {
            ckb_testkit::error!("failed to message_tx.send, error: {:?}", err);
        }
    }
}

fn batch_unbounded() -> (MessageTxs, MessageRxs) {
    let mut message_txs = MessageTxs::new();
    let mut message_rxs = MessageRxs::new();
    for peer_index in 0..100 {
        let (message_tx, message_rx) = unbounded();
        message_txs.insert(PeerIndex::new(peer_index), message_tx);
        message_rxs.insert(PeerIndex::new(peer_index), message_rx);
    }
    (message_txs, message_rxs)
}
