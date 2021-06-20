use crate::node::NodeOptions;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CaseOptions {
    pub make_all_nodes_out_of_ibd: bool,
    pub make_all_nodes_connected: bool,
    pub node_options: HashMap<&'static str, NodeOptions>,
}

impl Default for CaseOptions {
    fn default() -> Self {
        CaseOptions {
            make_all_nodes_out_of_ibd: true,
            make_all_nodes_connected: false,
            node_options: Default::default(),
        }
    }
}
