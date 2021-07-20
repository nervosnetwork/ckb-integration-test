use crate::testdata::{dump_testdata, Testdata};
use crate::{CKB2019, CKB2021};
use ckb_testkit::{Node, NodeOptions};

pub struct Height100000TestData;

impl Testdata for Height100000TestData {
    fn generate(&self) {
        let mut node2021 = {
            let node_options = NodeOptions {
                node_name: "node2021",
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/empty",
                chain_spec: "testdata/spec/ckb2021",
                app_config: "testdata/config/ckb2021",
            };
            Node::init("Height1000002V2TestData", node_options, true)
        };

        node2021.start();
        node2021.mine_to(100000);
        dump_testdata(node2021, "Height1000002V2TestData");
    }
}
