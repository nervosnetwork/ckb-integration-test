use crate::case::{Case, CaseOptions};
use crate::node::NodeOptions;
use crate::nodes::Nodes;
use crate::{CKB_V1_BINARY, CKB_V2_BINARY};

pub struct Networking;

impl Case for Networking {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_out_of_ibd: true,
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            node_options: vec![
                (
                    "ckb-v1",
                    NodeOptions {
                        ckb_binary: CKB_V1_BINARY.lock().clone(),
                        initial_database: "db/Height13TestData",
                        chain_spec: "config/ckb-v1",
                        app_config: "spec/ckb-v1",
                    },
                ),
                (
                    "ckb-v2",
                    NodeOptions {
                        ckb_binary: CKB_V2_BINARY.lock().clone(),
                        initial_database: "db/Height13TestData",
                        chain_spec: "config/ckb-v1",
                        app_config: "spec/ckb-v1",
                    },
                ),
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node_v1 = nodes.get_node("ckb-v1");
        let node_v2 = nodes.get_node("ckb-v2");
        node_v1.p2p_connect(node_v2);

        node_v1.mine(1);
        nodes.waiting_for_sync();
        node_v2.mine(1);
        nodes.waiting_for_sync();
    }
}
