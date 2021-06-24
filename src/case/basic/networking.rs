use crate::case::{Case, CaseOptions};
use crate::node::NodeOptions;
use crate::nodes::Nodes;
use crate::{CKB_FORK0_BINARY, CKB_FORK2021_BINARY};

pub struct BasicNetworking;

impl Case for BasicNetworking {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![
                NodeOptions {
                    node_name: "node-fork0",
                    ckb_binary: CKB_FORK0_BINARY.lock().clone(),
                    initial_database: "db/Height13TestData",
                    chain_spec: "spec/fork2021",
                    app_config: "config/fork2021",
                },
                NodeOptions {
                    node_name: "node-fork2021",
                    ckb_binary: CKB_FORK2021_BINARY.lock().clone(),
                    initial_database: "db/Height13TestData",
                    chain_spec: "spec/fork2021",
                    app_config: "config/fork2021",
                },
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node_fork0 = nodes.get_node("node-fork0");
        let node_fork2021 = nodes.get_node("node-fork2021");
        node_fork0.mine(10);
        node_fork2021.mine(10);
        node_fork0.p2p_connect(node_fork2021);

        node_fork0.mine(10);
        nodes.waiting_for_sync().expect("waiting for sync");
        node_fork2021.mine(10);
        nodes.waiting_for_sync().expect("waiting for sync");
    }
}
