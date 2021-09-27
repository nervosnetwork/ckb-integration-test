pub mod deployer;
pub mod frequently_used_instructions;
pub mod net_console;
pub mod run_case_helper;

pub use net_console::NetConsole;

use ckb_testkit::Node;
use ckb_types::core::{BlockNumber, EpochNumber};

// TODO rename to estimate_start_number_of_epoch
pub fn calc_epoch_start_number(node: &Node, epoch_number: EpochNumber) -> BlockNumber {
    assert!(node.consensus().permanent_difficulty_in_dummy);
    let genesis_epoch = node
        .rpc_client()
        .get_epoch_by_number(0)
        .expect("genesis epoch should exist");
    genesis_epoch.length.value() * epoch_number
}
