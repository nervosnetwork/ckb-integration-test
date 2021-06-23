use crate::node::NodeOptions;

// TODO Too dirty
#[derive(Debug, Clone)]
pub struct CaseOptions {
    pub make_all_nodes_connected: bool,
    pub make_all_nodes_synced: bool,
    pub make_all_nodes_connected_and_synced: bool,
    pub node_options: Vec<NodeOptions>,
}

impl Default for CaseOptions {
    fn default() -> Self {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: Default::default(),
        }
    }
}
