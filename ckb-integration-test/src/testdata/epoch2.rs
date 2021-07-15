use crate::testdata::{dump_testdata, Testdata};
use crate::{CKB2019, CKB2021};
use ckb_testkit::{Node, NodeOptions};

pub struct Epoch2TestData;

impl Testdata for Epoch2TestData {
    fn generate(&self) {
        let mut node2019 = {
            let node_options = NodeOptions {
                node_name: "node2019",
                ckb_binary: CKB2019.read().unwrap().clone(),
                initial_database: "testdata/db/empty",
                chain_spec: "testdata/spec/ckb2019",
                app_config: "testdata/config/ckb2019",
            };
            Node::init("Epoch2V1TestData", node_options, false)
        };
        let mut node2021 = {
            let node_options = NodeOptions {
                node_name: "node2021",
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/empty",
                chain_spec: "testdata/spec/ckb2021",
                app_config: "testdata/config/ckb2021",
            };
            Node::init("Epoch2V2TestData", node_options, true)
        };

        node2019.start();
        node2021.start();
        loop {
            node2019.mine(1);
            let tip = node2019.get_tip_block();
            node2021.submit_block(&tip);
            if tip.epoch().number() >= 2 {
                break;
            }
        }

        dump_testdata(node2019, "Epoch2V1TestData");
        dump_testdata(node2021, "Epoch2V2TestData");
    }
}
