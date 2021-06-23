use crate::case::{Case, CaseOptions};
use crate::node::NodeOptions;
use crate::nodes::Nodes;
use crate::{CKB_FORK0_BINARY, CKB_FORK2021_BINARY};

pub struct Networking;

impl Case for Networking {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![
                (
                    "ckb-fork0",
                    NodeOptions {
                        ckb_binary: CKB_FORK0_BINARY.lock().clone(),
                        initial_database: "db/Height13TestData",
                        chain_spec: "spec/ckb-fork0",
                        app_config: "config/ckb-fork0",
                    },
                ),
                (
                    "ckb-v2",
                    NodeOptions {
                        ckb_binary: CKB_FORK2021_BINARY.lock().clone(),
                        initial_database: "db/Height13TestData",
                        chain_spec: "spec/ckb-fork0",
                        app_config: "config/ckb-fork0",
                    },
                ),
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node_v1 = nodes.get_node("ckb-fork0");
        let node_v2 = nodes.get_node("ckb-v2");
        node_v1.p2p_connect(node_v2);

        node_v1.mine(1);
        nodes.waiting_for_sync();
        node_v2.mine(1);
        nodes.waiting_for_sync();
    }
}
