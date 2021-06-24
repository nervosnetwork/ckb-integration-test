use crate::node::Node;
use ckb_types::core::BlockNumber;

pub(super) fn median_timestamp(node: &Node, block_number: BlockNumber) -> u64 {
    let mut timestamps = (block_number - 37 + 1..=block_number)
        // let mut timestamps = (block_number - 37 ..block_number)
        .map(|number| node.get_block_by_number(number).timestamp())
        .collect::<Vec<_>>();
    timestamps.sort_unstable();
    timestamps[timestamps.len() >> 1]
}

pub(super) fn committed_timestamp(node: &Node, block_number: BlockNumber) -> u64 {
    node.get_block_by_number(block_number).timestamp()
}
