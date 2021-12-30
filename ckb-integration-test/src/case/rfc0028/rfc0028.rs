use super::{ERROR_IMMATURE, RFC0028_EPOCH_NUMBER};
use crate::prelude::*;
use crate::util::estimate_start_number_of_epoch;
use ckb_testkit::ckb_types::{
    core::TransactionView,
    packed::{CellInput, OutPoint},
};
use ckb_testkit::util::since_from_relative_timestamp;
use ckb_testkit::{assert_result_eq, BuildInstruction};

/// ## Convention
///
/// - Transaction's median time is less than or equal to its committed time
///
/// - `tx.input.since.metric_flag` is block timestamp (10)
///
/// - `tx.input.since.relative_flag` is relative (1)
///
/// - The timestamp verification requires the chain's timestamps must be
///   increasing. So my constructed chain is `block.timestamp = T + block.number`
///
/// ## Note
///
/// ## Cases Before Fork Switch
///
/// ```text
/// ┌───┬────────────────────┬──────────────────────┬────────────────────┬────────────────┐
/// │   │                    │ input tx number      │ tip number         │                │
/// │id │ since.relative_secs│ tx.median_time()     │ tip.median_time()  │  2019          │
/// ├───┼────────────────────┼──────────────────────┼────────────────────┼────────────────┤
/// │   │                    │ 1998                 │ 2999               │                │
/// │0  │ 1s                 │ T + 1980ms           │ T + 2981ms         │  Ok            │
/// ├───┼────────────────────┼──────────────────────┼────────────────────┼────────────────┤
/// │   │                    │ 1999                 │ 2999               │                │
/// │1  │ 1s                 │ T + 1981ms           │ T + 2981ms         │  Ok            │
/// ├───┼────────────────────┼──────────────────────┼────────────────────┼────────────────┤
/// │   │                    │ 2000                 │ 2999               │                │
/// │2  │ 1s                 │ T + 1982ms           │ T + 2981ms         │  Err(Immature) │
/// └───┴────────────────────┴──────────────────────┴────────────────────┴────────────────┘
/// ```
///
/// ## Cases After Fork Switch
///
/// ```text
/// ┌───┬────────────────────┬──────────────────────┬────────────────────┬────────────────┐
/// │   │                    │ input tx number      │ tip number         │                │
/// │id │ since.relative_secs│ tx.commit_time()     │ tip.median_time()  │  2021          │
/// ├───┼────────────────────┼──────────────────────┼────────────────────┼────────────────┤
/// │   │                    │ 1980                 │ 3000               │                │
/// │3  │ 1s                 │ T + 1980ms           │ T + 2982ms         │  Ok            │
/// ├───┼────────────────────┼──────────────────────┼────────────────────┼────────────────┤
/// │   │                    │ 1981                 │ 3000               │                │
/// │4  │ 1s                 │ T + 1981ms           │ T + 2982ms         │  Ok            │
/// ├───┼────────────────────┼──────────────────────┼────────────────────┼────────────────┤
/// │   │                    │ 1982                 │ 3000               │                │
/// │5  │ 1s                 │ T + 1982ms           │ T + 2982ms         │  Err(Immature) │
/// └───┴────────────────────┴──────────────────────┴────────────────────┴────────────────┘
/// ```
pub struct RFC0028;

impl Case for RFC0028 {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![NodeOptions {
                node_name: String::from("node2021"),
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/empty",
                chain_spec: "testdata/spec/ckb2021",
                app_config: "testdata/config/ckb2021",
            }],
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");
        let fork_switch_height = estimate_start_number_of_epoch(node2021, RFC0028_EPOCH_NUMBER);

        let relative_secs = 1;
        let since = since_from_relative_timestamp(relative_secs);

        let construct_chain_instructions = {
            let t = node2021.get_tip_block().timestamp();
            let mut ins = Vec::new();
            for number in node2021.get_tip_block_number() + 1..fork_switch_height {
                ins.push(BuildInstruction::HeaderTimestamp {
                    template_number: number,
                    timestamp: t + number,
                });
            }
            ins
        };
        let input_tx = {
            let input = node2021.get_spendable_always_success_cells()[0].to_owned();
            node2021.always_success_transaction(&input)
        };

        // Run cases 0, 1, 2.
        //
        // The current tip number is 2999, that has not activate fork.
        // Please read struct documentation for detail.
        //
        // [(case_id, input_tx_committed_number, tip_number)]
        let cases_before_switch = vec![
            (0, 1998, Ok(())),
            (1, 1999, Ok(())),
            (2, 2000, Err(ERROR_IMMATURE)),
        ];
        for (case_id, input_tx_committed_number, expected_result_before_switch) in
            cases_before_switch
        {
            let node = node2021.clone_node(&format!("case-{}-before-switch", case_id));

            let mut ins = construct_chain_instructions.clone();
            ins.extend(vec![
                BuildInstruction::Propose {
                    template_number: input_tx_committed_number - 2,
                    proposal_short_id: input_tx.proposal_short_id(),
                },
                BuildInstruction::Commit {
                    template_number: input_tx_committed_number,
                    transaction: input_tx.clone(),
                },
            ]);

            let tx = build_transaction(&node, &input_tx, since);
            ins.extend(vec![
                BuildInstruction::Propose {
                    template_number: fork_switch_height - 1 - 2,
                    proposal_short_id: tx.proposal_short_id(),
                },
                BuildInstruction::Commit {
                    template_number: fork_switch_height - 1,
                    transaction: tx,
                },
            ]);
            let actual_result_before_switch =
                node.build_according_to_instructions(fork_switch_height, ins);

            assert_result_eq!(
                expected_result_before_switch,
                actual_result_before_switch,
                "case-{}",
                case_id,
            );
        }

        // Run cases 4, 5, 6.
        //
        // The current tip number is 2999, that has not activate fork.
        // Please read struct documentation for detail.
        //
        // [(case_id, input_tx_committed_number, tip_number)]
        let cases_after_switch = vec![
            (3, 1980, Ok(())),
            (4, 1981, Ok(())),
            (5, 1982, Err(ERROR_IMMATURE)),
        ];
        for (case_id, input_tx_committed_number, expected_result_after_switch) in cases_after_switch
        {
            let node = node2021.clone_node(&format!("case-{}-after-switch", case_id));

            let mut ins = construct_chain_instructions.clone();
            ins.extend(vec![
                BuildInstruction::Propose {
                    template_number: input_tx_committed_number - 2,
                    proposal_short_id: input_tx.proposal_short_id(),
                },
                BuildInstruction::Commit {
                    template_number: input_tx_committed_number,
                    transaction: input_tx.clone(),
                },
            ]);

            let tx = build_transaction(&node, &input_tx, since);
            ins.extend(vec![
                BuildInstruction::Propose {
                    template_number: fork_switch_height - 2,
                    proposal_short_id: tx.proposal_short_id(),
                },
                BuildInstruction::Commit {
                    template_number: fork_switch_height,
                    transaction: tx,
                },
            ]);
            let actual_result_after_switch =
                node.build_according_to_instructions(fork_switch_height, ins);

            assert_result_eq!(
                expected_result_after_switch,
                actual_result_after_switch,
                "case-{}",
                case_id,
            );
        }
    }
}

fn build_transaction(node: &Node, input_tx: &TransactionView, since: u64) -> TransactionView {
    let placeholder_input = &node.get_spendable_always_success_cells()[0];
    node.always_success_transaction(placeholder_input)
        .as_advanced_builder()
        .set_inputs(vec![CellInput::new(
            OutPoint::new(input_tx.hash(), 0),
            since,
        )])
        .build()
}
