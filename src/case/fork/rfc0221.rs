use crate::case::{Case, CaseOptions};
use crate::node::{Node, NodeOptions};
use crate::nodes::Nodes;
use crate::util::since_from_relative_timestamp;
use crate::{CKB_FORK0_BINARY, CKB_FORK2021_BINARY};
use ckb_types::{
    core::{BlockNumber, EpochNumberWithFraction, TransactionBuilder},
    packed::{self, CellInput, CellOutput},
    prelude::*,
};
use std::thread::sleep;
use std::time::Duration;

// TODO enforce RFC0221_EPOCH be the same with `params.fork` in spec.toml
// TODO enforce RFC0221_EPOCH near by "db/Epoch2TestData"
// TODO Nodes in same case should have the same `initial_database`
// TODO Db version is related to ckb binary version. How to solve it?
pub const RFC0221_EPOCH: u64 = 1_979_121_768_857_602; // EpochNumberWithFraction::new(2, 50, 1800)

// 2. after  rfc0221, when tx is not mature for old rule, but mature for new rule,
//   node_v1 rejects tx, node_v2 accepts tx

// 3. after  rfc0221, when tx is mature for old rule and mature for old rule,
//   node_v1 and node_v2 accepts tx

// 1. before rfc0221, node_v1 and node_v2 reject tx, until it mature for old rule;

pub struct RFC0221BeforeSwitch;

impl Case for RFC0221BeforeSwitch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: true,
            make_all_nodes_synced: true,
            make_all_nodes_connected_and_synced: true,
            node_options: vec![
                (
                    "ckb-fork0",
                    NodeOptions {
                        ckb_binary: CKB_FORK0_BINARY.lock().clone(),
                        initial_database: "db/Epoch2V1TestData",
                        chain_spec: "spec/ckb-fork0",
                        app_config: "config/ckb-fork0",
                    },
                ),
                (
                    "ckb-fork2021",
                    NodeOptions {
                        ckb_binary: CKB_FORK2021_BINARY.lock().clone(),
                        initial_database: "db/empty",
                        chain_spec: "spec/ckb-fork2021",
                        app_config: "config/ckb-fork0",
                    },
                ),
            ]
            .into_iter()
            .collect(),
        }
    }

    // Before rfc0221, node_v1 and node_v2 use old rule: `since`'s start_time = median time of input's committed timestamp
    fn run(&self, nodes: Nodes) {
        let rfc0221_switch = EpochNumberWithFraction::from_full_value(RFC0221_EPOCH);
        let node_v1 = nodes.get_node("ckb-fork0");
        let node_v2 = nodes.get_node("ckb-fork2021");

        // Construct a transaction tx:
        //   - since: relative 2 seconds
        let relative_secs = 2;
        let relative_mills = relative_secs * 1000;
        let since = since_from_relative_timestamp(relative_secs);
        let input = &{
            let mut cells = node_v1.get_live_always_success_cells();
            cells.pop().expect("pop last cell")
        };
        let input_block_number = input
            .transaction_info
            .as_ref()
            .expect("live cell should have transaction info")
            .block_number;
        let start_time_of_old_rule = median_timestamp(node_v1, input_block_number);
        let tx = TransactionBuilder::default()
            .input(CellInput::new(input.out_point.clone(), since))
            .output(
                CellOutput::new_builder()
                    .lock(input.cell_output.lock())
                    .type_(input.cell_output.type_())
                    .capacity(input.capacity().pack())
                    .build(),
            )
            .output_data(Default::default())
            .cell_dep(node_v1.always_success_cell_dep())
            .build();

        loop {
            nodes.waiting_for_sync();

            let tip_number = node_v1.get_tip_block_number();
            let tip_median_time = median_timestamp(node_v1, tip_number);
            if start_time_of_old_rule + relative_mills <= tip_median_time {
                break;
            } else {
                let result = node_v1
                    .rpc_client()
                    .send_transaction_result(tx.pack().data().into());
                assert!(
                    result.is_err(),
                    "before rfc0221, tx is immature for node_v1 according to old rule and should be failed to submit, but got: {:?}",
                    result,
                );
                let result = node_v2
                    .rpc_client()
                    .send_transaction_result(tx.pack().data().into());
                assert!(
                    result.is_err(),
                    "before rfc0221, tx is immature for node_v2 according to old rule and should be failed to submit, but got: {:?}",
                    result,
                );
            }

            sleep(Duration::from_secs(1));
            node_v1.mine(1);
            assert!(node_v1.get_tip_block().epoch() < rfc0221_switch);
        }

        // Disconnect these nodes so that transactions will not be broadcast between them
        nodes.waiting_for_sync();
        node_v1.p2p_disconnect(node_v2);

        let result = node_v1
            .rpc_client()
            .send_transaction_result(tx.pack().data().into());
        assert!(
            result.is_ok(),
            "before rfc0221, tx is mature for node_v1 according to old rule and should be success to submit, but got: {:?}",
            result,
        );
        let result = node_v2
            .rpc_client()
            .send_transaction_result(tx.pack().data().into());
        assert!(
            result.is_ok(),
            "before rfc0221, tx is mature for node_v2 according to old rule and should be success to submit, but got: {:?}",
            result,
        );
    }
}

pub struct RFC0221AfterSwitch;

impl Case for RFC0221AfterSwitch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: true,
            make_all_nodes_synced: true,
            make_all_nodes_connected_and_synced: true,
            node_options: vec![
                (
                    "ckb-fork0",
                    NodeOptions {
                        ckb_binary: CKB_FORK0_BINARY.lock().clone(),
                        initial_database: "db/Epoch2V1TestData",
                        chain_spec: "spec/ckb-fork0",
                        app_config: "config/ckb-fork0",
                    },
                ),
                (
                    "ckb-fork2021",
                    NodeOptions {
                        ckb_binary: CKB_FORK2021_BINARY.lock().clone(),
                        initial_database: "db/Epoch2V2TestData",
                        chain_spec: "spec/ckb-fork2021",
                        app_config: "config/ckb-fork0",
                    },
                ),
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let rfc0221_switch = EpochNumberWithFraction::from_full_value(RFC0221_EPOCH);
        let node_v1 = nodes.get_node("ckb-fork0");
        let node_v2 = nodes.get_node("ckb-fork2021");
        let mine_one = || {
            let template = node_v1.rpc_client().get_block_template(None, None, None);
            assert!(template.transactions.is_empty());
            let block = packed::Block::from(template).into_view();
            node_v1.submit_block(&block);
            node_v2.submit_block(&block);
        };

        while node_v1.get_tip_block().epoch() < rfc0221_switch {
            mine_one();
        }
        for _ in 0..37 {
            mine_one();
            sleep(Duration::from_secs(1));
        }
        nodes.waiting_for_sync();

        // Construct a transaction tx:
        //   - since: relative 2 seconds
        let relative_secs = 2;
        let relative_millis = relative_secs * 1000;
        let since = since_from_relative_timestamp(relative_secs);
        let input = &{
            let mut cells = node_v1.get_live_always_success_cells();
            cells.pop().expect("pop last cell")
        };
        let input_block_number = input
            .transaction_info
            .as_ref()
            .expect("live cell should have transaction info")
            .block_number;
        let start_time_of_old_rule = median_timestamp(node_v1, input_block_number);
        let start_time_of_rfc0221 = committed_timestamp(node_v1, input_block_number);
        let tx = TransactionBuilder::default()
            .input(CellInput::new(input.out_point.clone(), since))
            .output(
                CellOutput::new_builder()
                    .lock(input.cell_output.lock())
                    .type_(input.cell_output.type_())
                    .capacity(input.capacity().pack())
                    .build(),
            )
            .output_data(Default::default())
            .cell_dep(node_v1.always_success_cell_dep())
            .build();

        // RFC0221 is looser than old rule
        assert!(start_time_of_old_rule < start_time_of_rfc0221);

        loop {
            let tip_number = node_v1.get_tip_block_number();
            let tip_median_time = median_timestamp(node_v1, tip_number);
            if start_time_of_rfc0221 + relative_millis <= tip_median_time {
                break;
            } else {
                // crate::error!("--------");
                // crate::error!("tip_median_time={}", tip_median_time);
                // crate::error!(
                //     "start_time_based_on_input_block_timestamp={}",
                //     start_time_of_rfc0221
                // );
                // crate::error!(
                //     "start_time_based_on_median_37={}",
                //     start_time_of_old_rule
                // );
                // crate::error!("start_time_based_on_input_block_timestamp - start_time_based_on_median_37 = {}", start_time_of_rfc0221 - start_time_of_old_rule);
                // crate::error!(
                //     "start_time_based_on_input_block_timestamp - tip_median_time = {}",
                //     start_time_of_rfc0221 - tip_median_time
                // );
                // crate::error!(
                //     "start_time_based_on_median_37 - tip_median_time = {}",
                //     start_time_of_old_rule - tip_median_time
                // );
                let result = node_v1
                    .rpc_client()
                    .send_transaction_result(tx.pack().data().into());
                assert!(
                    result.is_err(),
                    "after rfc0221, node_v1 should reject tx according to old rule, but got: {:?}",
                    result,
                );
                let result = node_v2
                    .rpc_client()
                    .send_transaction_result(tx.pack().data().into());
                assert!(
                    result.is_err(),
                    "after rfc0221, node_v2 should reject tx according to rfc0221, but got: {:?}",
                    result,
                );

                sleep(Duration::from_secs(1));
                mine_one();
            }
        }

        // Disconnect these nodes so that transactions will not be broadcast between them
        nodes.waiting_for_sync();
        node_v1.p2p_disconnect(node_v2);

        let result = node_v1
            .rpc_client()
            .send_transaction_result(tx.pack().data().into());
        assert!(
            result.is_err(),
            "after rfc0221, node_v1 should reject tx according to old rule, but got: {:?}",
            result,
        );
        let result = node_v2
            .rpc_client()
            .send_transaction_result(tx.pack().data().into());
        assert!(
            result.is_ok(),
            "after rfc0221, node_v2 should accept tx according to rfc0221, but got: {:?}",
            result,
        );
    }
}

fn median_timestamp(node: &Node, block_number: BlockNumber) -> u64 {
    let mut timestamps = (block_number - 37 + 1..=block_number)
        // let mut timestamps = (block_number - 37 ..block_number)
        .map(|number| node.get_block_by_number(number).timestamp())
        .collect::<Vec<_>>();
    timestamps.sort_unstable();
    timestamps[timestamps.len() >> 1]
}

fn committed_timestamp(node: &Node, block_number: BlockNumber) -> u64 {
    node.get_block_by_number(block_number).timestamp()
}
