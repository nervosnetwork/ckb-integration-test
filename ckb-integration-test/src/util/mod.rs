pub mod deployer;
pub mod shortcuts;

pub use shortcuts::{v0_100, v0_43};

use ckb_testkit::ckb_types::core::{BlockNumber, EpochNumber};
use ckb_testkit::Node;

pub fn estimate_start_number_of_epoch(node: &Node, epoch_number: EpochNumber) -> BlockNumber {
    assert!(node.consensus().permanent_difficulty_in_dummy);
    let genesis_epoch = node
        .rpc_client()
        .get_epoch_by_number(0)
        .expect("genesis epoch should exist");
    genesis_epoch.length.value() * epoch_number
}
