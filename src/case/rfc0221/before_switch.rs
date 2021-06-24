use crate::case::rfc0221::util::median_timestamp;
use crate::case::{Case, CaseOptions};
use crate::node::NodeOptions;
use crate::nodes::Nodes;
use crate::util::{since_from_relative_timestamp, wait_until};
use crate::{CKB_FORK0_BINARY, CKB_FORK2021_BINARY};
use ckb_types::{
    core::{EpochNumberWithFraction, TransactionBuilder},
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

pub struct RFC0221BeforeSwitch;

impl Case for RFC0221BeforeSwitch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: true,
            make_all_nodes_synced: true,
            make_all_nodes_connected_and_synced: true,
            node_options: vec![
                NodeOptions {
                    node_name: "ckb-fork0",
                    ckb_binary: CKB_FORK0_BINARY.lock().clone(),
                    initial_database: "db/Epoch2V1TestData",
                    chain_spec: "spec/ckb-fork0",
                    app_config: "config/ckb-fork0",
                },
                NodeOptions {
                    node_name: "ckb-fork2021",
                    ckb_binary: CKB_FORK2021_BINARY.lock().clone(),
                    initial_database: "db/empty",
                    chain_spec: "spec/ckb-fork2021",
                    app_config: "config/ckb-fork0",
                },
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
            // Use the last live cell as input to make sure the constructed
            // transaction cannot pass the "since verification" at short future
            node_v1.mine(1);
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
            nodes.waiting_for_sync().expect("waiting for sync");

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
                    "Before RFC0221, node_v1 should reject tx according to old rule, but got: {:?}",
                    result,
                );
                let result = node_v2
                    .rpc_client()
                    .send_transaction_result(tx.pack().data().into());
                assert!(
                    result.is_err(),
                    "Before RFC0221, node_v2 should reject tx according to old rule, but got: {:?}",
                    result,
                );
            }

            sleep(Duration::from_secs(1));
            node_v1.mine(1);
            assert!(node_v1.get_tip_block().epoch() < rfc0221_switch);
        }

        let sent = node_v1
            .rpc_client()
            .send_transaction_result(tx.pack().data().into());
        assert!(
            sent.is_ok(),
            "Before RFC0221, node_v1 should accept tx according to old rule, but got: {:?}",
            sent,
        );
        let synced = wait_until(10, || {
            node_v2
                .rpc_client()
                .send_transaction_result(tx.pack().data().into())
                .is_ok()
        });
        if !synced {
            let sent2 = node_v2
                .rpc_client()
                .send_transaction_result(tx.pack().data().into());
            assert!(
                sent2.is_ok(),
                "Before RFC0221, node_v2 should accept tx according to old rule, but got: {:?}",
                sent2,
            );
        }
    }
}
