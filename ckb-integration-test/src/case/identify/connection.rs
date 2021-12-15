use crate::preclude::*;
use crate::util::{v0_100, v0_43};
use ckb_testkit::{
    assert_result_eq,
    ckb_types::{packed, prelude::*},
    connector::{ConnectorBuilder, SharedState, SimpleProtocolHandler, SimpleServiceHandler},
    SupportProtocols,
};
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// This test case creates 3 connectors with 3 client versions:
///   - "0.43.0"
///   - "0.100.0"
///   - ""
///
/// The 3 connectors should be success to connect the CKB full node.
pub struct IdentifyConnection;

impl Case for IdentifyConnection {
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

impl IdentifyConnection {
    fn run_case(&self, node: &Node, case: &CaseParams) -> Result<(), String> {
        let network_identifier = {
            let consensus = node.consensus();
            let genesis_hash = format!("{:x}", consensus.genesis_hash);
            format!("/{}/{}", consensus.id, &genesis_hash[..8])
        };
        let shared = Arc::new(RwLock::new(SharedState::new()));
        let mut connector = ConnectorBuilder::new()
            .protocol_meta({
                SimpleProtocolHandler::new(Arc::clone(&shared), SupportProtocols::Sync).build(true)
            })
            .protocol_meta({
                SimpleProtocolHandler::new(Arc::clone(&shared), SupportProtocols::Identify)
                    .build(false)
            })
            .build(SimpleServiceHandler::new(Arc::clone(&shared)), shared);
        connector.connect(&node)?;

        if let Some(session) = connector.get_session(&node) {
            connector.send_identify_message(
                &node,
                &case.client_version,
                &network_identifier,
                vec![],
                session.address,
            )?;

            if let Ok(shared) = connector.shared().read() {
                if let Some(receiver) = shared
                    .get_protocol_receiver(&session.id, &SupportProtocols::Identify.protocol_id())
                {
                    let data = receiver
                        .recv_timeout(Duration::from_secs(10))
                        .expect("receive IdentifyMessage");
                    let _ = packed::IdentifyMessage::from_slice(data.as_ref())
                        .map_err(|err| format!("{:?}", err))?;
                }
            }
        }
        Ok(())
    }

    fn cases_params(&self) -> Vec<CaseParams> {
        vec![
            CaseParams {
                id: 0,
                client_version: v0_43(),
                expected_result: Ok(()),
            },
            CaseParams {
                id: 1,
                client_version: v0_100(),
                expected_result: Ok(()),
            },
            CaseParams {
                id: 2,
                client_version: "".to_string(),
                expected_result: Ok(()),
            },
        ]
    }
}

struct CaseParams {
    id: usize,
    client_version: String,
    expected_result: Result<(), String>,
}
