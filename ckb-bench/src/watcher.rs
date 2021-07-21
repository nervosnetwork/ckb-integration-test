use ckb_testkit::{Node, Nodes};
use ckb_types::core::{BlockNumber, HeaderView};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Metrics {
    pub tps: u64,
    pub average_block_time_ms: u64,
    pub average_block_transactions: u64,
    pub start_block_number: u64,
    pub end_block_number: u64,
    pub network_nodes: u64,
    pub bench_nodes: u64,
    pub total_transactions_size: u64,
}

/// Watcher watches the CKB node, it
/// - Judge whether the CKB is zero-load.
///   When the node's tx-pool is empty, and recent 20 blocks' transactions are empty, we consider
///   the node is zero-load.
/// - Judge whether the CKB is steady-load.
///   When the node's tip is 40 blocks far from zero-load-number, we consider the node is
///   steady-load.
pub struct Watcher {
    nodes: Nodes,
}

const N_BLOCKS: usize = 20;

impl Watcher {
    pub fn new(nodes: Nodes) -> Self {
        Self { nodes }
    }

    pub fn is_zero_load(&self) -> bool {
        self.nodes.nodes().all(|node| {
            let tx_pool_info = node.rpc_client().tx_pool_info();
            if tx_pool_info.total_tx_cycles.value() != 0 || tx_pool_info.total_tx_size.value() != 0
            {
                return false;
            }

            let mut number = node.get_tip_block_number();
            let mut n_recent_blocks = N_BLOCKS;
            while number > 0 && n_recent_blocks > 0 {
                let block = node.get_block_by_number(number);
                if block.transactions().len() > 1 {
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

    pub fn is_steady_load(&self, zero_load_number: BlockNumber) -> bool {
        zero_load_number + N_BLOCKS * 2 <= self.nodes.get_fixed_header().number()
    }

    pub fn calc_recent_metrics(&self, zero_load_number: BlockNumber) -> Metrics {
        let tip_fixed_number = self.nodes.get_fixed_header().number();
        let node = self.nodes.nodes().last().unwrap();

        let mut prefix_sum = 0;
        // [(block_number, timestamp, prefix_sum_txns)]
        let blocks_info: HashMap<BlockNumber, (u64, usize)> = (zero_load_number..=tip_fixed_number)
            .map(|number| {
                let block = node.get_block_by_number(number);
                prefix_sum += block.transactions();
                (block.number(), (block.timestamp(), prefix_sum))
            })
            .collect();

        let mut max_tps = 0;
        for number in zero_load_number..=tip_fixed_number {
            let (timestamp, prefix_sum_txns) = blocks_info.get(&number).unwrap();
            if number > N_BLOCKS as u64 && blocks_info.contains_key(&(number - N_BLOCKS)) {
                let (b_timestamp, b_prefix_sum_txns) =
                    blocks_info.get(&(number - N_BLOCKS)).unwrap();
                let tps = ((prefix_sum_txns - b_prefix_sum_txns) as f64
                    / (timestamp - b_timestamp) as f64) as u64;
                if tps > max_tps {
                    max_tps = tps;
                }
            }
        }
        Metrics {
            tps: max_tps,
            // TODO more metrics
            ..Default::default()
        }
    }
}
