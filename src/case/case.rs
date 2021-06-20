use crate::case::CaseOptions;
use crate::node::Node;
use crate::nodes::Nodes;
use std::collections::HashMap;

pub fn run_case(c: Box<dyn Case>) {
    println!("Start running case \"{}\"", c.case_name());
    let nodes = c.before_run();
    for node in nodes.nodes() {
        println!(
            "Started node, case_name: {}, node_name: {}, log_path: {}",
            node.case_name(),
            node.node_name(),
            node.log_path().display(),
        );
    }
    c.run(nodes);
    println!("End running case \"{}\"", c.case_name());
}

pub trait Case: Send {
    fn case_name(&self) -> &str {
        case_name(self)
    }

    fn case_options(&self) -> CaseOptions;

    fn before_run(&self) -> Nodes {
        let case_name = self.case_name();
        let mut nodes = HashMap::new();
        for (node_name, node_options) in self.case_options().node_options {
            let mut node = Node::init(case_name, node_name, node_options);
            node.start();
            nodes.insert(node_name.to_string(), node);
        }
        let nodes = Nodes::from(nodes);
        if self.case_options().make_all_nodes_out_of_ibd {
            for node in nodes.nodes() {
                node.mine(1);
            }
        }
        if self.case_options().make_all_nodes_connected {
            nodes.p2p_connect();
        }
        nodes
    }

    fn run(&self, nodes: Nodes);
}

fn case_name<T: ?Sized>(_: &T) -> &str {
    let type_name = ::std::any::type_name::<T>();
    type_name.split_terminator("::").last().unwrap()
}
