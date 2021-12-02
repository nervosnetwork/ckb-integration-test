use super::{ERROR_IMMATURE_HEADER, RFC0036_EPOCH_NUMBER};
use crate::preclude::*;
use crate::util::estimate_start_number_of_epoch;
use crate::util::run_case_helper::{run_case_after_switch, run_case_before_switch};
use ckb_testkit::ckb_types::core::TransactionView;

/// ```text
/// ┌──────────────┬───────────┬───────────┐
/// │              │           │           │
/// │ HeaderDep    │ v2019     │  v2021    │
/// ├──────────────┼───────────┼───────────┤
/// │              │           │           │
/// │ tip.header   │ Immature  │   Ok      │
/// │              │           │           │
/// └──────────────┴───────────┴───────────┘
/// ```
pub struct RFC0036;

impl Case for RFC0036 {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![NodeOptions {
                node_name: String::from("node2021"),
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/Epoch2V2TestData",
                chain_spec: "testdata/spec/cellbase_maturity_not_zero_2021",
                app_config: "testdata/config/ckb2021",
            }],
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");

        for node in nodes.nodes() {
            assert!(node.consensus().cellbase_maturity.value() > 0);
        }

        let fork_switch_height = estimate_start_number_of_epoch(node2021, RFC0036_EPOCH_NUMBER);
        node2021.mine_to(fork_switch_height - 6);

        // [(case_id, expected_result_before_switch, expected_result_after_switch)]
        let cases = vec![(0, Err(ERROR_IMMATURE_HEADER), Ok(()))];
        for (case_id, expected_result_before_switch, expected_result_after_switch) in cases {
            let tx = build_transaction(node2021);
            run_case_before_switch(
                node2021,
                fork_switch_height,
                case_id,
                vec![tx.clone()],
                expected_result_before_switch,
            );
            run_case_after_switch(
                node2021,
                fork_switch_height,
                case_id,
                vec![tx],
                expected_result_after_switch,
            );
        }
    }
}

fn build_transaction(node: &Node) -> TransactionView {
    let tip_hash = node.get_tip_block().hash();
    let header_dep = tip_hash;
    let input = node.get_spendable_always_success_cells()[0].to_owned();
    node.always_success_transaction(&input)
        .as_advanced_builder()
        .header_dep(header_dep)
        .build()
}
