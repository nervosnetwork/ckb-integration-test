use crate::case::{Case, CaseOptions};
use crate::node::{Node, NodeOptions};
use crate::nodes::Nodes;
use crate::util::since_from_relative_timestamp;
use crate::{CKB_V1_BINARY, CKB_V2_BINARY};
use ckb_types::{
    core::{BlockNumber, EpochNumberWithFraction, TransactionBuilder},
    packed::{CellInput, CellOutput},
    prelude::*,
};
use std::thread::sleep;
use std::time::Duration;

// TODO enforce RFC0221_EPOCH be the same with `params.fork` in spec.toml
// TODO enforce RFC0221_EPOCH near by "db/Epoch2TestData"
// TODO Nodes in same case should have the same `initial_database`
// TODO Db version is related to ckb binary version. How to solve it?
pub const RFC0221_EPOCH: u64 = 1_979_121_768_857_602; // EpochNumberWithFraction::new(2, 50, 1800)

// 1. before rfc0221, node_v1 and node_v2 reject tx, until it mature for old rule;

// 2. after  rfc0221, when tx is not mature for old rule, but mature for new rule,
//   node_v1 rejects tx, node_v2 accepts tx

// 3. after  rfc0221, when tx is mature for old rule and mature for old rule,
//   node_v1 and node_v2 accepts tx

// 1. before rfc0221, node_v1 and node_v2 reject tx, until it mature for old rule;

pub struct BeforeRFC0221Switch;

impl Case for BeforeRFC0221Switch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: true,
            make_all_nodes_synced: true,
            make_all_nodes_connected_and_synced: true,
            node_options: vec![
                (
                    "ckb-v1",
                    NodeOptions {
                        ckb_binary: CKB_V1_BINARY.lock().clone(),
                        initial_database: "db/Epoch2V1TestData",
                        chain_spec: "spec/ckb-v1",
                        app_config: "config/ckb-v1",
                    },
                ),
                (
                    "ckb-v2",
                    NodeOptions {
                        ckb_binary: CKB_V2_BINARY.lock().clone(),
                        initial_database: "db/empty",
                        chain_spec: "spec/rfc0221",
                        app_config: "config/ckb-v1",
                    },
                ),
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let rfc0221_switch = EpochNumberWithFraction::from_full_value(RFC0221_EPOCH);
        let node_v1 = nodes.get_node("ckb-v1");
        let node_v2 = nodes.get_node("ckb-v2");

        // Construct a transaction tx:
        //   - since: relative 2 seconds
        let relative_secs = 2;
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
        let start_time_based_on_median_37 = median_timestamp(node_v1, input_block_number);
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
            if start_time_based_on_median_37 + relative_secs * 1000 * 1000 <= tip_median_time {
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

fn median_timestamp(node: &Node, block_number: BlockNumber) -> u64 {
    let mut timestamps = (block_number - 37 + 1..=block_number)
        .map(|number| node.get_block_by_number(number).timestamp())
        .collect::<Vec<_>>();
    timestamps.sort_unstable();
    timestamps[timestamps.len() >> 1]
}

fn committed_timestamp(node: &Node, block_number: BlockNumber) -> u64 {
    node.get_block_by_number(block_number).timestamp()
}
//
// pub struct AfterRFC0221Switch;
//
// impl Case for AfterRFC0221Switch {
//     fn case_options(&self) -> CaseOptions {
//         CaseOptions {
//             make_all_nodes_connected: true,
//             make_all_nodes_synced: true,
//             make_all_nodes_connected_and_synced: true,
//             node_options: vec![
//                 (
//                     "ckb-v1",
//                     NodeOptions {
//                         ckb_binary: CKB_V1_BINARY.lock().clone(),
//                         initial_database: "db/Epoch2V1TestData",
//                         chain_spec: "spec/ckb-v1",
//                         app_config: "config/ckb-v1",
//                     },
//                 ),
//                 (
//                     "ckb-v2",
//                     NodeOptions {
//                         ckb_binary: CKB_V2_BINARY.lock().clone(),
//                         initial_database: "db/Epoch2V2TestData",
//                         chain_spec: "spec/rfc0221",
//                         app_config: "config/ckb-v1",
//                     },
//                 ),
//             ]
//             .into_iter()
//             .collect(),
//         }
//     }
//
//     fn run(&self, nodes: Nodes) {
//         let rfc0221_switch = EpochNumberWithFraction::from_full_value(RFC0221_EPOCH);
//         let node_v1 = nodes.get_node("ckb-v1");
//         let node_v2 = nodes.get_node("ckb-v2");
//
//         // Construct a transaction tx:
//         //   - since: relative 2 seconds
//         let relative_secs = 2;
//         let since = since_from_relative_timestamp(relative_secs);
//         let input = &{
//             let mut cells = node_v1.get_live_always_success_cells();
//             cells.pop().expect("pop last cell")
//         };
//         let input_block_number = input
//             .transaction_info
//             .as_ref()
//             .expect("live cell should have transaction info")
//             .block_number;
//         let start_time_based_on_median_37 = median_timestamp(node_v1, input_block_number);
//         let start_time_based_on_input_block_timestamp =
//             committed_timestamp(node_v1, input_block_number);
//         let tx1 = TransactionBuilder::default()
//             .input(CellInput::new(input.out_point.clone(), since))
//             .output(
//                 CellOutput::new_builder()
//                     .lock(input.cell_output.lock())
//                     .type_(input.cell_output.type_())
//                     .capacity(input.capacity().pack())
//                     .build(),
//             )
//             .output_data(Default::default())
//             .cell_dep(node_v1.always_success_cell_dep())
//             .build();
//
//         // RFC0221 is looser than old rule
//         assert!(start_time_based_on_median_37 < start_time_based_on_input_block_timestamp);
//
//         nodes.waiting_for_sync();
//         loop {
//             let tip_block = node_v1.get_tip_block();
//             let tip_median_time = median_timestamp(node_v1, tip_block.number());
//             assert!(tip_block.epoch() < rfc0221_switch);
//
//             if start_time_based_on_median_37 + relative_secs * 1000 * 1000 > tip_median_time {
//                 let result = node_v1
//                     .rpc_client()
//                     .send_transaction_result(tx1.pack().data().into());
//                 assert!(
//                     result.is_err(),
//                     "before rfc0221, tx is immature for node_v1 according to old rule and should be failed to submit, but got: {:?}",
//                     result,
//                 );
//                 let result = node_v2
//                     .rpc_client()
//                     .send_transaction_result(tx1.pack().data().into());
//                 assert!(
//                     result.is_err(),
//                     "before rfc0221, tx is immature for node_v2 according to old rule and should be failed to submit, but got: {:?}",
//                     result,
//                 );
//             } else {
//                 // Disconnect these nodes so that transactions will not be broadcast between them
//                 node_v1.p2p_disconnect(node_v2);
//
//                 let result = node_v1
//                     .rpc_client()
//                     .send_transaction_result(tx1.pack().data().into());
//                 assert!(
//                     result.is_ok(),
//                     "before rfc0221, tx is mature for node_v1 according to old rule and should be success to submit, but got: {:?}",
//                     result,
//                 );
//                 let result = node_v2
//                     .rpc_client()
//                     .send_transaction_result(tx1.pack().data().into());
//                 assert!(
//                     result.is_ok(),
//                     "before rfc0221, tx is mature for node_v2 according to old rule and should be success to submit, but got: {:?}",
//                     result,
//                 );
//
//                 // Re-connect nodes
//                 node_v1.p2p_connect(node_v2);
//                 break;
//             }
//
//             sleep(Duration::from_secs(1));
//             node_v1.mine(1);
//             nodes.waiting_for_sync();
//         }
//
//         // Ensure the blockchain rfc0221 come into effect
//         while node_v1.get_tip_block().epoch() < rfc0221_switch {
//             node_v1.mine(1);
//         }
//     }
// }
