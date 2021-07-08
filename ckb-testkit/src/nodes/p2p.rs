use crate::Nodes;

impl Nodes {
    pub fn p2p_connect(&self) {
        for node_a in self.nodes() {
            for node_b in self.nodes() {
                if node_a.p2p_address() != node_b.p2p_address() {
                    node_a.p2p_connect(node_b);
                }
            }
        }
    }

    pub fn p2p_disconnect(&self) {
        for node_a in self.nodes() {
            for node_b in self.nodes() {
                if node_a.p2p_address() != node_b.p2p_address() {
                    node_a.p2p_disconnect(node_b);
                }
            }
        }
    }
}
