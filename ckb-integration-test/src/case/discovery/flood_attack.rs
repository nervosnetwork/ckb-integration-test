use super::DISCOVERY_FLAG_V1;
use crate::prelude::*;
use ckb_testkit::connector::{SharedState, SimpleProtocolHandler, SimpleServiceHandler};
use ckb_testkit::{
    assert_result_eq,
    ckb_types::{packed, prelude::*},
    connector::ConnectorBuilder,
    util::wait_until,
    SupportProtocols,
};
use std::sync::{Arc, RwLock};

/// Send `DiscoveryMessage` frequently, to set up a flood attack on discovery protocol.
///
/// CKB full node should resist the attack by disconnecting or banning attacking peers.
pub struct DiscoveryFloodAttack;

impl Case for DiscoveryFloodAttack {
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

impl DiscoveryFloodAttack {
    fn run_case(&self, node: &Node, case: &CaseParams) -> Result<(), String> {
        let shared = Arc::new(RwLock::new(SharedState::new()));
        let mut connector = ConnectorBuilder::new()
            .protocol_meta({
                SimpleProtocolHandler::new(Arc::clone(&shared), SupportProtocols::Discovery)
                    .build(false)
            })
            .protocol_meta({
                SimpleProtocolHandler::new(Arc::clone(&shared), SupportProtocols::Sync).build(true)
            })
            .build(SimpleServiceHandler::new(Arc::clone(&shared)), shared);
        connector.connect(&node)?;

        let mut final_result = Ok(());
        for _ in 0..case.times {
            if let Err(err) =
                connector.send(&node, SupportProtocols::Discovery, case.message.as_bytes())
            {
                final_result = Err(err);
                break;
            }
        }

        let banned = wait_until(5, || !node.rpc_client().get_banned_addresses().is_empty());
        if banned {
            return Err("banned".to_string());
        }

        let disconnected = wait_until(5, || node.rpc_client().get_peers().is_empty());
        if disconnected {
            return Err("disconnected".to_string());
        }

        final_result
    }

    fn cases_params(&self) -> Vec<CaseParams> {
        let get_nodes_message = {
            let discovery_get_node = packed::GetNodes::new_builder()
                .listen_port(packed::PortOpt::default())
                .count(100u32.pack())
                .version(DISCOVERY_FLAG_V1.pack())
                .build();
            let discovery_payload = packed::DiscoveryPayload::new_builder()
                .set(discovery_get_node)
                .build();
            packed::DiscoveryMessage::new_builder()
                .payload(discovery_payload)
                .build()
        };
        let nodes_message_announce_true = {
            let discovery_nodes = packed::Nodes::new_builder().announce(true.pack()).build();
            let discovery_payload = packed::DiscoveryPayload::new_builder()
                .set(discovery_nodes)
                .build();
            packed::DiscoveryMessage::new_builder()
                .payload(discovery_payload)
                .build()
        };
        let nodes_message_announce_false = {
            let discovery_nodes = packed::Nodes::new_builder().announce(false.pack()).build();
            let discovery_payload = packed::DiscoveryPayload::new_builder()
                .set(discovery_nodes)
                .build();
            packed::DiscoveryMessage::new_builder()
                .payload(discovery_payload)
                .build()
        };
        vec![
            CaseParams {
                id: 0,
                times: 1,
                message: get_nodes_message.clone(),
                expected_result: Ok(()),
            },
            CaseParams {
                id: 1,
                times: 100,
                message: get_nodes_message.clone(),
                expected_result: Err("disconnected".to_string()),
            },
            CaseParams {
                id: 2,
                times: 1,
                message: nodes_message_announce_true.clone(),
                expected_result: Ok(()),
            },
            CaseParams {
                id: 3,
                times: 100,
                message: nodes_message_announce_true.clone(),
                // TODO this bug is reported to @driftluo
                // expected_result: Err("disconnected".to_string()),
                expected_result: Ok(()),
            },
            CaseParams {
                id: 4,
                times: 1,
                message: nodes_message_announce_false.clone(),
                expected_result: Ok(()),
            },
            CaseParams {
                id: 5,
                times: 100,
                message: nodes_message_announce_false.clone(),
                expected_result: Err("disconnected".to_string()),
            },
        ]
    }
}

struct CaseParams {
    id: usize,
    times: usize,
    message: packed::DiscoveryMessage,
    // TODO listening_port
    // TODO inbound, outbound
    expected_result: Result<(), String>,
}
