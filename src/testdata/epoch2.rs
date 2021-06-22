use crate::node::{Node, NodeOptions};
use crate::testdata::{dump_testdata, Testdata};
use crate::CKB_V1_BINARY;

pub struct Epoch2TestData;

impl Testdata for Epoch2TestData {
    fn generate(&self) {
        let node_options = NodeOptions {
            ckb_binary: CKB_V1_BINARY.lock().clone(),
            initial_database: "db/empty",
            chain_spec: "spec/ckb-v1",
            app_config: "config/ckb-v1",
        };
        let mut node = Node::init(self.testdata_name(), self.testdata_name(), node_options);
        node.start();
        loop {
            node.mine(1);
            let tip = node.get_tip_block();
            if tip.epoch().number() >= 2 {
                break;
            }
        }
        dump_testdata(node, self.testdata_name());
    }
}
