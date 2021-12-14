use super::{ERROR_IMMATURE_HEADER, RFC0036_EPOCH_NUMBER};
use crate::preclude::*;
use crate::util::estimate_start_number_of_epoch;
use ckb_testkit::ckb_types::core::{BlockNumber, TransactionView};
use ckb_testkit::{assert_result_eq, BuildInstruction};

const RFC0036_BLOCK_NUMBER: BlockNumber = 3000;

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

        for case in self.cases_params() {
            let node = {
                let node = node2021.clone_node(&format!("case-{}-node", case.id));
                node.pull_node(node2021).unwrap();
                node
            };
            let tx = self.build_transaction(&node);
            let ins = vec![
                BuildInstruction::Propose {
                    proposal_short_id: tx.proposal_short_id(),
                    template_number: case.height - 2,
                },
                BuildInstruction::Commit {
                    transaction: tx,
                    template_number: case.height,
                },
            ];
            let actual_result = node.build_according_to_instructions(case.height, ins);
            assert_result_eq!(
                case.expected_result,
                actual_result,
                "case.id: {}, node.log: {}",
                case.id,
                node.log_path().to_string_lossy(),
            );
        }
    }
}

#[derive(Debug)]
struct CaseParams {
    id: usize,
    height: BlockNumber,
    expected_result: Result<(), &'static str>,
}

impl RFC0036 {
    fn cases_params(&self) -> Vec<CaseParams> {
        vec![
            CaseParams {
                id: 0,
                height: RFC0036_BLOCK_NUMBER - 1,
                expected_result: Err(ERROR_IMMATURE_HEADER),
            },
            CaseParams {
                id: 1,
                height: RFC0036_BLOCK_NUMBER,
                expected_result: Ok(()),
            },
        ]
    }

    fn build_transaction(&self, node: &Node) -> TransactionView {
        let tip_hash = node.get_tip_block().hash();
        let header_dep = tip_hash;
        let input = node.get_spendable_always_success_cells()[0].to_owned();
        node.always_success_transaction(&input)
            .as_advanced_builder()
            .header_dep(header_dep)
            .build()
    }
}
