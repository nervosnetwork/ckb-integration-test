use crate::node::{Node, NodeOptions};
use crate::testdata::{dump_testdata, Testdata};
use crate::CKB2019;

pub struct Height13TestData;

impl Testdata for Height13TestData {
    fn generate(&self) {
        let node_options = NodeOptions {
            node_name: "node2019",
            ckb_binary: CKB2019.read().unwrap().clone(),
            initial_database: "db/empty",
            chain_spec: "spec/ckb2021",
            app_config: "config/ckb2021",
        };
        let mut node = Node::init(self.testdata_name(), node_options);
        node.start();
        node.mine(13);
        dump_testdata(node, self.testdata_name());
    }
}
