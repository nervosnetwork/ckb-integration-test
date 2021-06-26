use crate::case::{Case, CaseOptions};
use crate::node::{Node, NodeOptions};
use crate::nodes::Nodes;
use crate::util::{
    since_from_absolute_epoch_number_with_fraction, since_from_relative_epoch_number_with_fraction,
};
use crate::CKB2021;
use ckb_types::{
    core::{cell::CellMeta, EpochNumber, EpochNumberWithFraction, TransactionBuilder},
    packed::{CellInput, CellOutput},
    prelude::*,
};

const RFC0223_EPOCH_NUMBER: EpochNumber = 3;

pub struct RFC0223AfterSwitch;

impl Case for RFC0223AfterSwitch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![NodeOptions {
                node_name: "node2021",
                ckb_binary: CKB2021.lock().clone(),
                initial_database: "db/Epoch2V2TestData",
                chain_spec: "spec/ckb2021",
                app_config: "config/ckb2021",
            }]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");

        while !is_rfc0223_switched(node2021) {
            node2021.mine(1);
        }

        let current_block_epoch = node2021.get_tip_block().epoch();
        let cells = node2021.get_live_always_success_cells();
        assert!(cells.len() >= 4);

        let build_transaction = |since: u64, input: &CellMeta| {
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
                .cell_dep(node2021.always_success_cell_dep())
                .build()
        };
        let since_relative_epoch_number_with_fraction1 =
            since_from_relative_epoch_number_with_fraction(EpochNumberWithFraction::new_unchecked(
                0, 1801, 1800,
            ));
        let since_relative_epoch_number_with_fraction2 =
            since_from_relative_epoch_number_with_fraction(EpochNumberWithFraction::new_unchecked(
                0, 1800, 1800,
            ));
        let since_absolute_epoch_number_with_fraction1 =
            since_from_absolute_epoch_number_with_fraction(EpochNumberWithFraction::new_unchecked(
                current_block_epoch.number(),
                2,
                1,
            ));
        let since_absolute_epoch_number_with_fraction2 =
            since_from_absolute_epoch_number_with_fraction(EpochNumberWithFraction::new_unchecked(
                current_block_epoch.number(),
                1,
                1,
            ));
        let txs = vec![
            build_transaction(since_relative_epoch_number_with_fraction1, &cells[0]),
            build_transaction(since_relative_epoch_number_with_fraction2, &cells[1]),
            build_transaction(since_absolute_epoch_number_with_fraction1, &cells[2]),
            build_transaction(since_absolute_epoch_number_with_fraction2, &cells[3]),
        ];

        // Move forward to make sure our since values become valid
        node2021.mine(1800 + 10);

        txs.iter().enumerate().for_each(|(i, tx)| {
            let result = node2021
                .rpc_client()
                .send_transaction_result(tx.pack().data().into());
            assert!(
                result
                    .as_ref()
                    .unwrap_err()
                    .to_string()
                    .contains("InvalidSince"),
                "node2021 should reject tx-{} according to rfc0223, but got: {:?}",
                i,
                result
            );
        });
    }
}

fn is_rfc0223_switched(node: &Node) -> bool {
    node.rpc_client().get_current_epoch().number.value() >= RFC0223_EPOCH_NUMBER
}
