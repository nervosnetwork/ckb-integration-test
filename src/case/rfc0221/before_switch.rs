use crate::case::rfc0221::util::median_timestamp;
use crate::case::{Case, CaseOptions};
use crate::node::{Node, NodeOptions};
use crate::nodes::Nodes;
use crate::util::{since_from_relative_timestamp, wait_until};
use crate::{CKB_FORK0_BINARY, CKB_FORK2021_BINARY};
use ckb_types::{
    core::{EpochNumber, TransactionBuilder},
    packed::{CellInput, CellOutput},
    prelude::*,
};
use std::thread::sleep;
use std::time::Duration;

// TODO enforce RFC0221_EPOCH_NUMBER be the same with `params.fork` in spec.toml, and greater then `db/EpochTestdata`
// TODO Nodes in same case should have the same `initial_database`
// TODO Db version is related to ckb binary version. How to solve it?
const RFC0221_EPOCH_NUMBER: EpochNumber = 3;

pub struct RFC0221BeforeSwitch;

impl Case for RFC0221BeforeSwitch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: true,
            make_all_nodes_synced: true,
            make_all_nodes_connected_and_synced: true,
            node_options: vec![
                NodeOptions {
                    node_name: "node-fork0",
                    ckb_binary: CKB_FORK0_BINARY.lock().clone(),
                    initial_database: "db/Epoch2V1TestData",
                    chain_spec: "spec/fork2021",
                    app_config: "config/fork2021",
                },
                NodeOptions {
                    node_name: "node-fork2021",
                    ckb_binary: CKB_FORK2021_BINARY.lock().clone(),
                    initial_database: "db/empty",
                    chain_spec: "spec/fork2021",
                    app_config: "config/fork2021",
                },
            ]
            .into_iter()
            .collect(),
        }
    }

    // Before rfc0221, node_fork0 and node_fork2021 use old rule: `since`'s start_time = median time of input's committed timestamp
    fn run(&self, nodes: Nodes) {
        let node_fork0 = nodes.get_node("node-fork0");
        let node_fork2021 = nodes.get_node("node-fork2021");

        // Construct a transaction tx:
        //   - since: relative 2 seconds
        let relative_secs = 2;
        let relative_mills = relative_secs * 1000;
        let since = since_from_relative_timestamp(relative_secs);
        let input = &{
            // Use the last live cell as input to make sure the constructed
            // transaction cannot pass the "since verification" at short future
            node_fork0.mine(1);
            let mut cells = node_fork0.get_live_always_success_cells();
            cells.pop().expect("pop last cell")
        };
        let input_block_number = input
            .transaction_info
            .as_ref()
            .expect("live cell should have transaction info")
            .block_number;
        let start_time_of_old_rule = median_timestamp(node_fork0, input_block_number);
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
            .cell_dep(node_fork0.always_success_cell_dep())
            .build();

        loop {
            nodes.waiting_for_sync().expect("waiting for sync");

            let tip_number = node_fork0.get_tip_block_number();
            let tip_median_time = median_timestamp(node_fork0, tip_number);
            if start_time_of_old_rule + relative_mills <= tip_median_time {
                break;
            } else {
                let result = node_fork0
                    .rpc_client()
                    .send_transaction_result(tx.pack().data().into());
                assert!(
                    result.is_err(),
                    "Before RFC0221, node_fork0 should reject tx according to old rule, but got: {:?}",
                    result,
                );
                let result = node_fork2021
                    .rpc_client()
                    .send_transaction_result(tx.pack().data().into());
                assert!(
                    result.is_err(),
                    "Before RFC0221, node_fork2021 should reject tx according to old rule, but got: {:?}",
                    result,
                );
            }

            sleep(Duration::from_secs(1));
            node_fork0.mine(1);
            assert!(!is_rfc0221_switched(node_fork0));
        }

        let sent = node_fork0
            .rpc_client()
            .send_transaction_result(tx.pack().data().into());
        assert!(
            sent.is_ok(),
            "Before RFC0221, node_fork0 should accept tx according to old rule, but got: {:?}",
            sent,
        );
        let synced = wait_until(10, || {
            node_fork2021
                .rpc_client()
                .send_transaction_result(tx.pack().data().into())
                .is_ok()
        });
        if !synced {
            let sent2 = node_fork2021
                .rpc_client()
                .send_transaction_result(tx.pack().data().into());
            assert!(
                sent2.is_ok(),
                "Before RFC0221, node_fork2021 should accept tx according to old rule, but got: {:?}",
                sent2,
            );
        }
    }
}

fn is_rfc0221_switched(node: &Node) -> bool {
    node.rpc_client().get_current_epoch().number.value() >= RFC0221_EPOCH_NUMBER
}
