use ckb_jsonrpc_types::HeaderView;
use crate::nodes::Nodes;

/// Watcher watches the CKB node, it
/// - Judge whether the CKB is zero-load.
///   When the node's tx-pool is empty, and recent 20 blocks' transactions are empty, we consider
///   the node is zero-load.
/// - Judge whether the CKB is steady-load.
///   When the node's tip is 5 blocks far from zero-load-number, we consider the node is
///   steady-load.
pub struct Watcher {
    nodes: Nodes,
}

const N_BLOCKS: usize = 5;

impl Watcher {
    pub fn new(nodes: Nodes) -> Self {
        Self { nodes }
    }

    pub fn is_zero_load(&self) -> bool {
        self.nodes.nodes().all(|node| {
            let tx_pool_info = node.rpc_client().tx_pool_info().unwrap();
            // TODO FIXME tx-pool stat issue
            // if tx_pool_info.total_tx_cycles.value() != 0 || tx_pool_info.total_tx_size.value() != 0
            // {
            //     return false;
            // }
            if tx_pool_info.pending.value() != 0
                || tx_pool_info.proposed.value() != 0
                || tx_pool_info.orphan.value() != 0
            {
                return false;
            }

            let mut number = node.rpc_client().get_tip_block_number().unwrap().value();
            let mut n_recent_blocks = N_BLOCKS;
            while number > 0 && n_recent_blocks > 0 {
                let block = node.rpc_client().get_block_by_number(number.into()).unwrap().unwrap();
                if block.transactions.len() > 1 {
                    return false;
                }

                number -= 1;
                n_recent_blocks -= 1;
            }

            number > 0 && n_recent_blocks == 0
        })
    }

    pub fn get_fixed_header(&self) -> HeaderView {
        self.nodes.get_fixed_header()
    }
}
