use ckb_testkit::Node;
use ckb_types::core::BlockNumber;
use serde_derive::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Metrics {
    pub from_block_number: BlockNumber,
    pub to_block_number: BlockNumber,
    pub total_transactions: usize,
    pub total_transactions_size: usize,
    pub transactions_per_second: u64,
    pub transactions_size_per_second: u64,
    pub average_block_transactions: usize,
    pub average_block_transactions_size: usize,
    pub average_block_time_ms: u64,
    pub n_nodes: usize,
    pub n_outputs: usize,
    pub ckb_version: String,
}

pub fn stat(
    node: &Node,
    from_number: BlockNumber,
    to_number: BlockNumber,
    stat_time: Duration,
) -> Metrics {
    assert_ne!(from_number, 0);
    assert!(from_number < to_number);
    let mut i = from_number;
    let mut j = from_number;
    let mut total_transactions = 0;
    let mut total_transactions_size = 0;
    let mut n_outputs = 0;
    let mut best_metrics = Metrics::default();
    loop {
        let block_i = node.get_block_by_number(i);
        let mut block_j_timestamp = 0;
        while j <= to_number {
            let block_j = node.get_block_by_number(j);
            block_j_timestamp = block_j.timestamp();
            if block_j.timestamp().saturating_sub(block_i.timestamp())
                >= stat_time.as_millis() as u64
            {
                break;
            }
            total_transactions += block_j.transactions().len();
            total_transactions_size += block_j.data().serialized_size_without_uncle_proposals();
            if n_outputs == 0 {
                if block_j.transactions().len() > 1 {
                    n_outputs = block_j.transactions()[1].outputs().len();
                }
            }
            j += 1;
        }

        if j > to_number {
            j = to_number;
        }

        let header_j = node.get_header_by_number(j);
        let tps = (total_transactions as f64 * 1000.0
            / (block_j_timestamp.saturating_sub(block_i.timestamp())) as f64)
            as u64;
        let sps = (total_transactions_size as f64 * 1000.0
            / (block_j_timestamp.saturating_sub(block_i.timestamp())) as f64)
            as u64;
        if tps > best_metrics.transactions_per_second {
            best_metrics = Metrics {
                from_block_number: block_i.number(),
                to_block_number: header_j.number(),
                total_transactions,
                total_transactions_size,
                transactions_per_second: tps,
                transactions_size_per_second: sps,
                average_block_transactions: total_transactions
                    / ((header_j.number() - block_i.number() + 1) as usize),
                average_block_transactions_size: total_transactions_size
                    / ((header_j.number() - block_i.number() + 1) as usize),
                average_block_time_ms: header_j.timestamp().saturating_sub(block_i.timestamp())
                    / (header_j.number() - block_i.number() + 1),
                ..Default::default()
            };
        }
        if j >= to_number {
            break;
        }

        total_transactions -= block_i.transactions().len();
        i += 1;
    }

    let local_node_info = node.rpc_client().local_node_info();
    best_metrics.ckb_version = local_node_info.version;
    best_metrics.n_nodes = local_node_info.connections.value() as usize + 1;
    best_metrics.n_outputs = n_outputs;
    best_metrics
}
