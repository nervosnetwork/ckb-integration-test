use crate::node::{Node, NodeOptions};
use crate::testdata::{dump_testdata, Testdata};
use crate::CKB_V1_BINARY;

pub struct Height13TestData;

impl Testdata for Height13TestData {
    fn generate(&self) {
        let node_options = NodeOptions {
            ckb_binary: CKB_V1_BINARY.lock().clone(),
            initial_database: "db/empty",
            chain_spec: "config/ckb-v1",
            app_config: "spec/ckb-v1",
        };
        let mut node = Node::init(self.testdata_name(), self.testdata_name(), node_options);
        node.start();
        node.mine(13);
        dump_testdata(node, self.testdata_name());
    }
}
