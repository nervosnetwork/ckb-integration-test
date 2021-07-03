// ## Cases And Expect Results
//
// After fork2021, nodes2019s disconnect node2021s, node2021s disconnect node2019s.
//
// Check the connections via RPC `get_peers`

use crate::case::{Case, CaseOptions};
use crate::node::{Node, NodeOptions};
use crate::nodes::Nodes;
use crate::util::wait_until;
use crate::{CKB2019, CKB2021};
use ckb_types::core::EpochNumber;

const RFC0234_EPOCH_NUMBER: EpochNumber = 3;

pub struct RFC0234AfterSwitchConnection;

impl Case for RFC0234AfterSwitchConnection {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: true,
            make_all_nodes_synced: true,
            make_all_nodes_connected_and_synced: true,
            node_options: vec![
                NodeOptions {
                    node_name: "node2019",
                    ckb_binary: CKB2019.read().unwrap().clone(),
                    initial_database: "db/Epoch2V1TestData",
                    chain_spec: "spec/ckb2019",
                    app_config: "config/ckb2019",
                },
                NodeOptions {
                    node_name: "node2019_2",
                    ckb_binary: CKB2019.read().unwrap().clone(),
                    initial_database: "db/Epoch2V1TestData",
                    chain_spec: "spec/ckb2019",
                    app_config: "config/ckb2019",
                },
                NodeOptions {
                    node_name: "node2021",
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "db/Epoch2V2TestData",
                    chain_spec: "spec/ckb2021",
                    app_config: "config/ckb2021",
                },
                NodeOptions {
                    node_name: "node2021_2",
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "db/Epoch2V2TestData",
                    chain_spec: "spec/ckb2021",
                    app_config: "config/ckb2021",
                },
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");
        loop {
            node2021.mine(1);
            if !is_rfc0234_switched(node2021) {
                nodes
                    .waiting_for_sync()
                    .expect("nodes should be synced before rfc0234.switch")
            } else {
                break;
            }
        }

        let disconnect_different_version_nodes = wait_until(20, || {
            nodes.nodes().all(|node| {
                let local_node_info = node.rpc_client().local_node_info();
                node.rpc_client()
                    .get_peers()
                    .iter()
                    .all(|peer| local_node_info.version == peer.version)
            })
        });
        if !disconnect_different_version_nodes {
            for node in nodes.nodes() {
                let local_node_info = node.rpc_client().local_node_info();
                for peer in node.rpc_client().get_peers() {
                    if local_node_info.version != peer.version {
                        panic!(
                            "nodes with different fork versions should be disconnected, but {}({}) still connect with {}({})",
                            node.node_name(), local_node_info.version, peer.node_id, peer.version,
                        );
                    }
                }
            }
        }

        // TODO Actually, the below check is for SyncProtocol
        let mut fresh_node2021 = {
            let node_options = NodeOptions {
                node_name: "fresh_node2021",
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "db/empty",
                chain_spec: "spec/ckb2021",
                app_config: "config/ckb2021",
            };
            Node::init(self.case_name(), node_options)
        };
        fresh_node2021.start();
        fresh_node2021.p2p_connect(node2021);
        let synced = wait_until(180, || {
            fresh_node2021.get_tip_block_number() == node2021.get_tip_block_number()
        });
        assert!(
            synced,
            "fresh_node2021 should sync from node2021s, fresh_node2021.tip: {}, node2021.tip: {}",
            fresh_node2021.get_tip_block_number(),
            node2021.get_tip_block_number(),
        );
    }
}

fn is_rfc0234_switched(node: &Node) -> bool {
    node.rpc_client().get_current_epoch().number.value() >= RFC0234_EPOCH_NUMBER
}
