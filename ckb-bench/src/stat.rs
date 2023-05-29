use serde_derive::{Deserialize, Serialize};
use std::time::Duration;
use ckb_types::core::{BlockNumber, BlockView, HeaderView};
use crate::node::Node;

/// On-chain report
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Report {
    /// Number of running CKB nodes
    pub n_nodes: usize,
    /// Number of transaction inputs and outputs
    pub n_inout: usize,
    /// Client version of the running CKB nodes
    pub ckb_version: String,
    /// Delay time between sending continuous transactions, equal to `--tx-interval-ms`
    pub delay_time_ms: Option<u64>,

    /// The chain height when starting benchmark
    pub from_block_number: BlockNumber,
    /// The chain height when ending benchmark
    pub to_block_number: BlockNumber,

    /// On-chain transactions per seconds
    pub transactions_per_second: u64,
    /// On-chain transaction size per seconds
    pub transactions_size_per_second: u64,

    /// Average block transactions
    pub average_block_transactions: usize,
    /// Average block transactions size
    pub average_block_transactions_size: usize,
    /// Average block interval in milliseconds
    pub average_block_time_ms: u64,

    /// Total transactions
    pub total_transactions: usize,
    /// Total transactions size
    pub total_transactions_size: usize,
}

pub fn stat(
    node: &Node,
    from_number: BlockNumber,
    to_number: BlockNumber,
    stat_time: Duration,
    delay_time: Option<Duration>,
) -> Report {
    assert_ne!(from_number, 0);
    assert!(from_number < to_number);
    let mut i = from_number;
    let mut j = from_number;
    let mut total_transactions = 0;
    let mut total_transactions_size = 0;
    let mut n_inout = 0;
    let mut best_report = Report::default();
    loop {
        let block_i = {
            let block = node.rpc_client().get_block_by_number(i.into()).unwrap();
            BlockView::from(block.unwrap())
        };

        let mut block_j_timestamp = 0;
        while j <= to_number {
            let block_j = {
                let block = node.rpc_client().get_block_by_number(j.into()).unwrap();
                BlockView::from(block.unwrap())
            };
            block_j_timestamp = block_j.timestamp();
            if block_j.timestamp().saturating_sub(block_i.timestamp())
                >= stat_time.as_millis() as u64
            {
                break;
            }
            total_transactions += block_j.transactions().len();
            total_transactions_size += block_j.data().serialized_size_without_uncle_proposals();
            if n_inout == 0 {
                if block_j.transactions().len() > 1 {
                    n_inout = block_j.transactions()[1].outputs().len();
                }
            }
            j += 1;
        }

        if j > to_number {
            j = to_number;
        }

        let header_j = {
            let block = node.rpc_client().get_header_by_number(j.into()).unwrap();
            HeaderView::from(block.unwrap())
        };
        let tps = (total_transactions as f64 * 1000.0
            / (block_j_timestamp.saturating_sub(block_i.timestamp())) as f64)
            as u64;
        let sps = (total_transactions_size as f64 * 1000.0
            / (block_j_timestamp.saturating_sub(block_i.timestamp())) as f64)
            as u64;
        if tps > best_report.transactions_per_second {
            best_report = Report {
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

    let local_node_info = node.rpc_client().local_node_info().unwrap();
    best_report.ckb_version = local_node_info.version;
    best_report.n_nodes = local_node_info.connections.value() as usize + 1;
    best_report.n_inout = n_inout;
    best_report.delay_time_ms = delay_time.map(|t| t.as_millis() as u64);
    best_report
}
