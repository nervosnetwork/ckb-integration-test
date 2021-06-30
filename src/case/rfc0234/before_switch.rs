use crate::case::{Case, CaseOptions};
use crate::node::{Node, NodeOptions};
use crate::nodes::Nodes;
use crate::{CKB2019, CKB2021};
use ckb_types::core::EpochNumber;

const RFC0234_EPOCH_NUMBER: EpochNumber = 3;

pub struct RFC0234BeforeSwitch;

impl Case for RFC0234BeforeSwitch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![
                NodeOptions {
                    node_name: "node2019",
                    ckb_binary: CKB2019.read().unwrap().clone(),
                    initial_database: "db/Epoch2V1TestData",
                    chain_spec: "spec/ckb2021",
                    app_config: "config/ckb2021",
                },
                // NodeOptions {
                //     node_name: "node2019_2",
                //     ckb_binary: CKB2019.read().unwrap().clone(),
                //     initial_database: "db/Epoch2V1TestData",
                //     chain_spec: "spec/ckb2021",
                //     app_config: "config/ckb2021",
                // },
                NodeOptions {
                    node_name: "node2021",
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "db/Epoch2V2TestData",
                    chain_spec: "spec/ckb2021",
                    app_config: "config/ckb2021",
                },
                // NodeOptions {
                //     node_name: "node2021_2",
                //     ckb_binary: CKB2021.read().unwrap().clone(),
                //     initial_database: "db/Epoch2V2TestData",
                //     chain_spec: "spec/ckb2021",
                //     app_config: "config/ckb2021",
                // },
            ]
                .into_iter()
                .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2019 = nodes.get_node("node2019");
        let node2021 = nodes.get_node("node2021");

        // node2019 mines, other nodes grow up via SyncProtocol
        nodes.p2p_disconnect();
        node2019.mine(10);
        assert!(!is_rfc0234_switched(node2019));
        nodes.p2p_connect();
        nodes.waiting_for_sync().expect("nodes should be synced");

        // node2021 mines, other nodes grow up via SyncProtocol
        nodes.p2p_disconnect();
        node2021.mine(10);
        assert!(!is_rfc0234_switched(node2021));
        nodes.p2p_connect();
        nodes.waiting_for_sync().expect("nodes should be synced");

        // node2019 mines, other nodes grow up via RelayProtocol
        nodes.p2p_connect();
        node2019.mine(1);
        nodes.waiting_for_sync().expect("nodes should be synced");

        // node2021 mines, other nodes grow up via RelayProtocol
        nodes.p2p_connect();
        node2021.mine(1);
        nodes.waiting_for_sync().expect("nodes should be synced");
    }
}

fn is_rfc0234_switched(node: &Node) -> bool {
    node.rpc_client().get_current_epoch().number.value() >= RFC0234_EPOCH_NUMBER
}