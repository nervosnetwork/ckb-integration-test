use crate::preclude::*;
use ckb_testkit::connector::{SharedState, SimpleProtocolHandler, SimpleServiceHandler};
use ckb_testkit::{
    assert_result_eq, connector::ConnectorBuilder, p2p::secio::SecioKeyPair, SupportProtocols,
};
use std::sync::{Arc, RwLock};

/// The CKB full node identifies peers with identical key pairs.
/// If multiple peers use the same key pair, CKB full node considers
/// the latter peers as malicious and rejects corresponding sessions.
///
/// In this test case, two connectors share the same key pair.
pub struct IdentifyIdenticalKeyPair;

impl Case for IdentifyIdenticalKeyPair {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![NodeOptions {
                node_name: String::from("node2021"),
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/empty",
                chain_spec: "testdata/spec/ckb2021",
                app_config: "testdata/config/ckb2021",
            }],
        }
    }

    fn run(&self, nodes: Nodes) {
        let template_node = nodes.get_node("node2021");
        template_node.mine(1);
        for case in self.cases_params() {
            let node = template_node.clone_node(&format!("{}-case-{}", self.case_name(), case.id));
            let actual_result = self.run_case(&node, &case);
            assert_result_eq!(
                case.expected_result,
                actual_result,
                "case.id={}, node.log=\"{}\"",
                case.id,
                node.log_path().to_string_lossy()
            );
        }
    }
}

impl IdentifyIdenticalKeyPair {
    fn run_case(&self, node: &Node, _case: &CaseParams) -> Result<(), String> {
        let key_pair = SecioKeyPair::secp256k1_generated();
        let shared1 = Arc::new(RwLock::new(SharedState::new()));
        let shared2 = Arc::new(RwLock::new(SharedState::new()));
        let mut connector1 = ConnectorBuilder::new()
            .protocol_meta({
                SimpleProtocolHandler::new(Arc::clone(&shared1), SupportProtocols::Sync).build(true)
            })
            .protocol_meta({
                SimpleProtocolHandler::new(Arc::clone(&shared1), SupportProtocols::Identify)
                    .build(false)
            })
            .key_pair(key_pair.clone())
            .build(
                SimpleServiceHandler::new(Arc::clone(&shared1)),
                Arc::clone(&shared1),
            );
        let mut connector2 = ConnectorBuilder::new()
            .protocol_meta({
                SimpleProtocolHandler::new(Arc::clone(&shared2), SupportProtocols::Sync).build(true)
            })
            .protocol_meta({
                SimpleProtocolHandler::new(Arc::clone(&shared2), SupportProtocols::Identify)
                    .build(false)
            })
            .key_pair(key_pair.clone())
            .build(
                SimpleServiceHandler::new(Arc::clone(&shared2)),
                Arc::clone(&shared2),
            );
        connector1.connect(&node)?;
        connector2.connect(&node)?;
        Ok(())
    }

    fn cases_params(&self) -> Vec<CaseParams> {
        vec![CaseParams {
            id: 0,
            expected_result: Err("timeout".to_string()),
        }]
    }
}

struct CaseParams {
    id: usize,
    expected_result: Result<(), String>,
}
