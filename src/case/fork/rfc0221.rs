use crate::case::{Case, CaseOptions};
use crate::node::NodeOptions;
use crate::nodes::Nodes;
use crate::util::since_from_relative_timestamp;
use crate::{CKB_V1_BINARY, CKB_V2_BINARY};
use ckb_types::{
    core::{EpochNumberWithFraction, TransactionBuilder},
    packed::{CellInput, CellOutput},
    prelude::*,
};

// TODO enforce it in spec.toml
// TODO enforce RFC0221_EPOCH near by "db/Epoch2TestData"
pub const RFC0221_EPOCH: u64 = 1_979_121_768_857_602; // EpochNumberWithFraction::new(2, 50, 1800)

pub struct RFC0221;

impl Case for RFC0221 {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_out_of_ibd: true,
            make_all_nodes_connected: true,
            make_all_nodes_synced: true,
            node_options: vec![
                (
                    "ckb-v1",
                    NodeOptions {
                        ckb_binary: CKB_V1_BINARY.lock().clone(),
                        initial_database: "db/Epoch2TestData",
                        chain_spec: "config/ckb-v1",
                        app_config: "spec/ckb-v1",
                    },
                ),
                (
                    "ckb-v2",
                    NodeOptions {
                        ckb_binary: CKB_V2_BINARY.lock().clone(),
                        initial_database: "db/Epoch2TestData",
                        chain_spec: "config/ckb-v1",
                        app_config: "spec/rfc0221",
                    },
                ),
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let rfc0221_epoch = EpochNumberWithFraction::from_full_value(RFC0221_EPOCH);
        let node_v1 = nodes.get_node("ckb-v1");
        let node_v2 = nodes.get_node("ckb-v2");

        // 1. before rfc0221, node_v1 and node_v2 reject tx, until it mature for old rule;

        // 2. after  rfc0221, when tx is not mature for old rule, but mature for new rule,
        //   node_v1 rejects tx, node_v2 accepts tx

        // 3. after  rfc0221, when tx is mature for old rule and mature for old rule,
        //   node_v1 and node_v2 accepts tx

        // 1. before rfc0221, node_v1 and node_v2 reject tx, until it mature for old rule;
        let cells = node_v1.get_live_always_success_cells();
        let input = &cells[cells.len() - 1];
        let relative_secs = 2;
        let since = since_from_relative_timestamp(relative_secs);
        let tx = {
            TransactionBuilder::default()
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
                .build()
        };
        let input_block_number = input
            .transaction_info
            .expect("live cell should have transaction info")
            .block_number;
        let start_time_based_on_median_37 = {
            let mut timestamps = (input_block_number - 37 + 1..=input_block_number)
                .map(|number| node_v1.get_block_by_number(number).timestamp())
                .collect::<Vec<_>>();
            timestamps.sort_unstable();
            timestamps[timestamps.len() >> 1]
        };
        let start_time_based_on_input_block_timestamp =
            node_v1.get_block_by_number(input_block_number).timestamp();

        loop {
            let tip_block = node_v1.get_tip_block();
            // let tip_epoch = tip_block.epoch();
            node_v1.rpc_client().send_transaction(tx.into())
        }
    }
}
