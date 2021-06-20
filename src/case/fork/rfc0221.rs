use crate::case::{Case, CaseOptions};
use crate::node::NodeOptions;
use crate::nodes::Nodes;
use crate::{CKB_V1_BINARY, CKB_V2_BINARY};

pub struct RFC0221;

impl Case for RFC0221 {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_out_of_ibd: true,
            make_all_nodes_connected: false,
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

    fn run(&self, _nodes: Nodes) {}
}
