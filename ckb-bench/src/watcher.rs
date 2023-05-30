use std::thread::sleep;
use std::time::{Duration, Instant};
use ckb_jsonrpc_types::{HeaderView, TxPoolInfo};
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

pub struct NodeStatus{
    node_id:String,
    tx_pool_info:TxPoolInfo
}

const N_BLOCKS: usize = 5;

impl Watcher {
    pub fn new(nodes: Nodes) -> Self {
        Self { nodes }
    }

    pub fn check_statue(
         &self,
        log_duration: u64,
        t_bench: Duration
    ) {
        let start_time = Instant::now();

        loop {

            let nodes_status = self.nodes.nodes().map(|node|{
                let raw_tx_pool = node.rpc_client().tx_pool_info().unwrap();
                NodeStatus{
                    node_id: node.node_name().into(),
                    tx_pool_info: raw_tx_pool,
                }
            });
            if self.nodes.nodes().len() > 1{
                println!()
            }
            nodes_status.for_each(|status|
                {
                    crate::info!("[node] node_id:{:?}, tip_number:{:?}, pool msg: pending :{:?},orphan:{:?},proposed: {:?} ",
                    status.node_id,
                    status.tx_pool_info.tip_number.value(),
                    status.tx_pool_info.pending.value(),
                    status.tx_pool_info.orphan.value(),
                    status.tx_pool_info.proposed.value());
                }
            );
            sleep(Duration::from_secs(log_duration));
            if start_time.elapsed() > t_bench {
                break;
            }


        }



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
