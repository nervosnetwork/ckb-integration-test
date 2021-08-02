use ckb_testkit::NodeOptions;
use std::path::PathBuf;

pub mod bench;
pub mod mine;
pub mod prepare;

pub(self) fn node_options() -> Vec<NodeOptions> {
    vec![
        NodeOptions {
            node_name: String::from("node2021_1"),
            ckb_binary: PathBuf::from("ckb"),
            initial_database: "testdata/db/empty",
            chain_spec: "testdata/spec/ckb2021",
            app_config: "testdata/config/ckb2021",
        },
        NodeOptions {
            node_name: String::from("node2021_2"),
            ckb_binary: PathBuf::from("ckb"),
            initial_database: "testdata/db/empty",
            chain_spec: "testdata/spec/ckb2021",
            app_config: "testdata/config/ckb2021",
        },
        NodeOptions {
            node_name: String::from("node2021_3"),
            ckb_binary: PathBuf::from("ckb"),
            initial_database: "testdata/db/empty",
            chain_spec: "testdata/spec/ckb2021",
            app_config: "testdata/config/ckb2021",
        },
    ]
}
