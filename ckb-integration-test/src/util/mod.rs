use ckb_types::core::{BlockNumber, EpochNumber};
use ckb_testkit::Node;

pub fn calc_epoch_start_number(node: &Node, epoch_number: EpochNumber) -> BlockNumber {
    assert!(node.consensus().permanent_difficulty_in_dummy);
    let genesis_epoch= node.rpc_client().get_epoch_by_number(0).expect("genesis epoch should exist");
    genesis_epoch.number.value() * epoch_number
}
