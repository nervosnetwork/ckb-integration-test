use crate::testdata::{dump_testdata, Testdata};
use crate::CKB2019;
use ckb_testkit::node::{Node, NodeOptions};

pub struct Height13TestData;

impl Testdata for Height13TestData {
    fn generate(&self) {
        let node_options = NodeOptions {
            node_name: "node2019",
            ckb_binary: CKB2019.read().unwrap().clone(),
            initial_database: "testdata/db/empty",
            chain_spec: "testdata/spec/ckb2019",
            app_config: "testdata/config/ckb2019",
        };
        let mut node = Node::init(self.testdata_name(), node_options, false);
        node.start();
        node.mine(13);
        dump_testdata(node, self.testdata_name());
    }
}
