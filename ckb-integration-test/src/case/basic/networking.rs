use crate::case::{Case, CaseOptions};
use crate::{CKB2019, CKB2021};
use ckb_testkit::NodeOptions;
use ckb_testkit::Nodes;

pub struct BasicNetworking;

impl Case for BasicNetworking {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![
                NodeOptions {
                    node_name: String::from("node2019"),
                    ckb_binary: CKB2019.read().unwrap().clone(),
                    initial_database: "testdata/db/Height13TestData",
                    chain_spec: "testdata/spec/ckb2019",
                    app_config: "testdata/config/ckb2019",
                },
                NodeOptions {
                    node_name: String::from("node2021"),
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "testdata/db/Height13TestData",
                    chain_spec: "testdata/spec/ckb2021",
                    app_config: "testdata/config/ckb2021",
                },
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2019 = nodes.get_node("node2019");
        let node2021 = nodes.get_node("node2021");
        node2019.mine(10);
        node2021.mine(10);
        node2019.p2p_connect(node2021);

        node2019.mine(10);
        nodes.waiting_for_sync().expect("waiting for sync");
        node2021.mine(10);
        nodes.waiting_for_sync().expect("waiting for sync");
    }
}
