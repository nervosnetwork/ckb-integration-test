use crate::prelude::*;
use ckb_testkit::connector::{SharedState, SimpleProtocolHandler, SimpleServiceHandler};
use ckb_testkit::{
    assert_result_eq,
    ckb_types::{packed, prelude::*},
    connector::ConnectorBuilder,
    p2p::multiaddr::Multiaddr,
    util::wait_until,
    SupportProtocols,
};
use std::sync::{Arc, RwLock};

/// Send empty or oversize DiscoveryMessage
pub struct ManipulatedAddresses;

impl Case for ManipulatedAddresses {
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

impl ManipulatedAddresses {
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
        connector.send(&node, SupportProtocols::Discovery, case.message.as_bytes())?;

        let banned = wait_until(5, || !node.rpc_client().get_banned_addresses().is_empty());
        if banned {
            return Err("banned".to_string());
        }

        let disconnected = wait_until(5, || node.rpc_client().get_peers().is_empty());
        if disconnected {
            return Err("disconnected".to_string());
        }

        Ok(())
    }

    fn cases_params(&self) -> Vec<CaseParams> {
        let make_nodes = |count: usize| {
            let nodes_vec = (0..count)
                .map(|port| {
                    let address: Multiaddr =
                        format!("/ip4/127.0.0.1/tcp/{}", port).parse().unwrap();
                    packed::Node::new_builder()
                        .addresses(vec![address.to_vec().pack()].pack())
                        .build()
                })
                .collect::<Vec<_>>();
            packed::NodeVec::new_builder().set(nodes_vec).build()
        };
        let nodes_message_1000_nodes_false_announce = {
            let discovery_nodes = packed::Nodes::new_builder()
                .announce(false.pack())
                .items(make_nodes(1000))
                .build();
            let discovery_payload = packed::DiscoveryPayload::new_builder()
                .set(discovery_nodes)
                .build();
            packed::DiscoveryMessage::new_builder()
                .payload(discovery_payload)
                .build()
        };
        let nodes_message_5000_nodes_true_announce = {
            let discovery_nodes = packed::Nodes::new_builder()
                .items(make_nodes(5000))
                .announce(true.pack())
                .build();
            let discovery_payload = packed::DiscoveryPayload::new_builder()
                .set(discovery_nodes)
                .build();
            packed::DiscoveryMessage::new_builder()
                .payload(discovery_payload)
                .build()
        };
        let nodes_message_5000_nodes_false_announce = {
            let discovery_nodes = packed::Nodes::new_builder()
                .announce(false.pack())
                .items(make_nodes(5000))
                .build();
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
                message: nodes_message_1000_nodes_false_announce.clone(),
                expected_result: Ok(()),
            },
            CaseParams {
                id: 1,
                message: nodes_message_5000_nodes_true_announce.clone(),
                expected_result: Err("disconnected".to_string()),
            },
            CaseParams {
                id: 2,
                message: nodes_message_5000_nodes_false_announce.clone(),
                expected_result: Err("disconnected".to_string()),
            },
        ]
    }
}

struct CaseParams {
    id: usize,
    message: packed::DiscoveryMessage,
    // TODO listening_port
    // TODO inbound, outbound
    expected_result: Result<(), String>,
}
