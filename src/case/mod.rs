mod basic;
mod case_options;
mod rfc0221;

use crate::node::Node;
use crate::nodes::Nodes;
use std::collections::HashMap;

pub use case_options::CaseOptions;

pub fn all_cases() -> Vec<Box<dyn Case>> {
    vec![
        Box::new(basic::networking::BasicNetworking),
        Box::new(rfc0221::before_switch::RFC0221BeforeSwitch),
        Box::new(rfc0221::after_switch::RFC0221AfterSwitch),
        Box::new(rfc0221::networking::RFC0221Networking),
    ]
}

pub fn run_case(case: Box<dyn Case>) {
    use crate::{info, CASE_NAME};
    CASE_NAME.with(|c| {
        *c.borrow_mut() = case.case_name().to_string();
    });

    info!("********** START **********");
    let nodes = case.before_run();
    case.run(nodes);
    info!("********** END **********");
}

fn case_name<T: ?Sized>(_: &T) -> &str {
    let type_name = ::std::any::type_name::<T>();
    type_name.split_terminator("::").last().unwrap()
}

pub trait Case: Send {
    fn case_name(&self) -> &str {
        case_name(self)
    }

    fn case_options(&self) -> CaseOptions;

    fn before_run(&self) -> Nodes {
        let case_name = self.case_name();
        let case_options = self.case_options();
        let mut nodes = HashMap::new();
        let mut first_node_name = None;
        for node_options in case_options.node_options.iter() {
            let mut node = Node::init(case_name, node_options.clone());
            let node_name = node.node_name().to_string();
            node.start();
            nodes.insert(node_name.clone(), node);
            if first_node_name.is_none() {
                first_node_name = Some(node_name);
            }
        }
        let nodes = Nodes::from(nodes);
        if case_options.make_all_nodes_connected_and_synced {
            for node in nodes.nodes() {
                node.mine(1);
            }
            nodes.p2p_connect();
            let any_node = nodes.get_node(first_node_name.as_ref().unwrap());
            any_node.mine(1);
            nodes.waiting_for_sync().expect("waiting for sync");
        } else {
            if case_options.make_all_nodes_connected {
                nodes.p2p_connect();
            }
            if case_options.make_all_nodes_synced {
                let any_node = nodes.get_node(first_node_name.as_ref().unwrap());
                any_node.mine(1);
                let tip_block = any_node.get_tip_block();
                for node in nodes.nodes() {
                    if node.node_name() != any_node.node_name() {
                        node.submit_block(&tip_block);
                    }
                }
                nodes.waiting_for_sync().expect("waiting for sync");
            }
        }
        nodes
    }

    fn run(&self, nodes: Nodes);
}
