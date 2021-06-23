use crate::case::CaseOptions;
use crate::node::Node;
use crate::nodes::Nodes;
use std::collections::HashMap;

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
        for (node_name, node_options) in case_options.node_options.iter() {
            let mut node = Node::init(case_name, node_name, node_options.clone());
            node.start();
            nodes.insert(node_name.to_string(), node);
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
            let any_node = nodes.get_node(first_node_name.unwrap());
            any_node.mine(1);
            nodes.waiting_for_sync();
        } else {
            if case_options.make_all_nodes_connected {
                nodes.p2p_connect();
            }
            if case_options.make_all_nodes_synced {
                let any_node = nodes.get_node(first_node_name.unwrap());
                any_node.mine(1);
                let tip_block = any_node.get_tip_block();
                for node in nodes.nodes() {
                    if node.node_name() != any_node.node_name() {
                        node.submit_block(&tip_block);
                    }
                }
                nodes.waiting_for_sync();
            }
        }
        nodes
    }

    fn run(&self, nodes: Nodes);
}

fn case_name<T: ?Sized>(_: &T) -> &str {
    let type_name = ::std::any::type_name::<T>();
    type_name.split_terminator("::").last().unwrap()
}
