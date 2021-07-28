use ckb_testkit::Node;
use ckb_types::core::BlockNumber;
use serde_derive::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Metrics {
    pub transactions_per_second: u64,
    // pub average_block_transactions: u64,
    // pub average_block_time: Duration,
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
    let mut best_tps = 0;
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
            j += 1;
        }

        let tps = (total_transactions as f64 * 1000.0
            / (block_j_timestamp.saturating_sub(block_i.timestamp())) as f64)
            as u64;
        if tps > best_tps {
            best_tps = tps;
        }
        if j > to_number {
            break;
        }

        total_transactions -= block_i.transactions().len();
        i += 1;
    }

    Metrics {
        transactions_per_second: best_tps,
    }
}
