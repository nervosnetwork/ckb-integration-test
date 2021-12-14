use super::{
    ERROR_DUPLICATE_CELL_DEPS, ERROR_MULTIPLE_MATCHES, RFC0029_BLOCK_NUMBER, RFC0029_EPOCH_NUMBER,
};
use crate::preclude::*;
use crate::util::deployer::Deployer;
use ckb_testkit::ckb_types::{
    core::{
        cell::CellMeta, BlockNumber, Capacity, DepType, ScriptHashType, TransactionBuilder,
        TransactionView,
    },
    packed::{Byte32, CellDep, CellInput, CellOutput, OutPointVec, Script},
    prelude::*,
};
use ckb_testkit::{assert_result_eq, BuildInstruction};

#[derive(Debug)]
struct CaseParams {
    id: usize,
    height: BlockNumber,
    script_hash_type: ScriptHashType,
    cell_deps: Vec<&'static str>,
    expected_result: Result<(), &'static str>,
}

/// ### Convention
///
/// - `a1`, `a2` and `b1` are 3 cells
///
/// - `a1`, `a2` and `b1` have the same type-script
///
/// - `a1` and `a2` have the same output-data
///
/// - `Group(x, y, ..)` indicates a `DepGroup` points to `x` and `y` cells
///
/// - when `script.hash_type` is `"data"`, `script.code_hash` is always `a1.data_hash`;
///   when `script.hash_type` is `"type"`, `script.code_hash` is always `a1.type_hash`
///
/// ## Cases
///
/// ```text
/// ┌────────┬────────────┬────────────────────────────────────┬────────────────────────┬───────────────────────┐
/// │        │            │                                    │                        │                       │
/// │    id  │   hash_type│    cell_deps                       │  2019                  │ 2021                  │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │     0  │    "data"  │    [a1]                            │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │     1  │    "data"  │    [a1, a1]                        │  Err(DuplicateDeps)    │ Err(DuplicateDeps)    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │     2  │    "data"  │    [a1, a2]                        │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │     3  │    "data"  │    [a1, b1]                        │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │     4  │    "data"  │    [Group(a1)]                     │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │     5  │    "data"  │    [Group(a1, a1)]                 │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │     6  │    "data"  │    [Group(a1, a2)]                 │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │     7  │    "data"  │    [Group(a1, b1)]                 │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │     8  │    "data"  │    [Group(a1), a1]                 │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │     9  │    "data"  │    [Group(a1), a2]                 │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    10  │    "data"  │    [Group(a1), b1]                 │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    11  │    "data"  │    [Group(a1), Group(a2)]          │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    12  │    "data"  │    [Group(a1), Group(b1)]          │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    13  │    "type"  │    [a1]                            │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    14  │    "type"  │    [a1, a1]                        │  Err(DuplicateDeps)    │ Err(DuplicateDeps)    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    15  │    "type"  │    [a1, a2]                        │  Err(MultipleMatches)  │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    16  │    "type"  │    [a1, b1]                        │  Err(MultipleMatches)  │ Err(MultipleMatches)  │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    17  │    "type"  │    [Group(a1)]                     │  Ok                    │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    18  │    "type"  │    [Group(a1, a1)]                 │  Err(MultipleMatches)  │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    19  │    "type"  │    [Group(a1, a2)]                 │  Err(MultipleMatches)  │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    20  │    "type"  │    [Group(a1, b1)]                 │  Err(MultipleMatches)  │ Err(MultipleMatches)  │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    21  │    "type"  │    [Group(a1), a1]                 │  Err(MultipleMatches)  │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    22  │    "type"  │    [Group(a1), a2]                 │  Err(MultipleMatches)  │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    23  │    "type"  │    [Group(a1), b1]                 │  Err(MultipleMatches)  │ Err(MultipleMatches)  │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    24  │    "type"  │    [Group(a1), Group(a2)]          │  Err(MultipleMatches)  │ Ok                    │
/// ├────────┼────────────┼────────────────────────────────────┼────────────────────────┼───────────────────────┤
/// │    25  │    "type"  │    [Group(a1), Group(b1)]          │  Err(MultipleMatches)  │ Err(MultipleMatches)  │
/// │        │            │                                    │                        │                       │
/// └────────┴────────────┴────────────────────────────────────┴────────────────────────┴───────────────────────┘
/// ```
pub struct RFC0029;

impl Case for RFC0029 {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![NodeOptions {
                node_name: String::from("node2021"),
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/Epoch2V2TestData",
                chain_spec: "testdata/spec/ckb2021",
                app_config: "testdata/config/ckb2021",
            }],
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");

        // We use this as type script of our deployed cells,
        // so that we can reference it via `ScriptHashType::Type`
        let type_script = node2021
            .always_success_script()
            .as_builder()
            .args("no-matter".pack())
            .build();

        // Deploy dependent cells
        let mut deployer = Deployer::new();
        // deploy "a1"
        {
            let output_data =
                include_bytes!("../../../testdata/spec/ckb2021/cells/always_success").pack();
            let output = CellOutput::new_builder()
                .lock(node2021.always_success_script())
                .type_(Some(type_script.clone()).pack())
                .build_exact_capacity(Capacity::bytes(output_data.len()).unwrap())
                .unwrap();
            deployer.deploy(node2021, "a1", output, output_data)
        }
        // deploy "a2"
        {
            let output_data =
                include_bytes!("../../../testdata/spec/ckb2021/cells/always_success").pack();
            let output = CellOutput::new_builder()
                .lock(node2021.always_success_script())
                .type_(Some(type_script.clone()).pack())
                .build_exact_capacity(Capacity::bytes(output_data.len()).unwrap())
                .unwrap();
            deployer.deploy(node2021, "a2", output, output_data)
        }
        // deploy "b1"
        {
            let output_data =
                include_bytes!("../../../testdata/spec/ckb2021/cells/another_always_success")
                    .pack();
            let output = CellOutput::new_builder()
                .lock(node2021.always_success_script())
                .type_(Some(type_script.clone()).pack())
                .build_exact_capacity(Capacity::bytes(output_data.len()).unwrap())
                .unwrap();
            deployer.deploy(node2021, "b1", output, output_data)
        }
        // deploy Group("a1"), naming "group_a1"
        {
            let output_data = OutPointVec::new_builder()
                .set(vec![deployer.get_out_point("a1")])
                .build()
                .as_bytes()
                .pack();
            let output = CellOutput::new_builder()
                .lock(node2021.always_success_script())
                .build_exact_capacity(Capacity::bytes(output_data.len()).unwrap())
                .unwrap();
            deployer.deploy(node2021, "group_a1", output, output_data)
        }
        // deploy Group("a2"), naming "group_a2"
        {
            let output_data = OutPointVec::new_builder()
                .set(vec![deployer.get_out_point("a2")])
                .build()
                .as_bytes()
                .pack();
            let output = CellOutput::new_builder()
                .lock(node2021.always_success_script())
                .build_exact_capacity(Capacity::bytes(output_data.len()).unwrap())
                .unwrap();
            deployer.deploy(node2021, "group_a2", output, output_data)
        }
        // deploy Group("b1"), naming "group_b1"
        {
            let output_data = OutPointVec::new_builder()
                .set(vec![deployer.get_out_point("b1")])
                .build()
                .as_bytes()
                .pack();
            let output = CellOutput::new_builder()
                .lock(node2021.always_success_script())
                .build_exact_capacity(Capacity::bytes(output_data.len()).unwrap())
                .unwrap();
            deployer.deploy(node2021, "group_b1", output, output_data)
        }
        // deploy Group("a1", "a1"), naming "group_a1_a1"
        {
            let output_data = OutPointVec::new_builder()
                .set(vec![
                    deployer.get_out_point("a1"),
                    deployer.get_out_point("a1"),
                ])
                .build()
                .as_bytes()
                .pack();
            let output = CellOutput::new_builder()
                .lock(node2021.always_success_script())
                .build_exact_capacity(Capacity::bytes(output_data.len()).unwrap())
                .unwrap();
            deployer.deploy(node2021, "group_a1_a1", output, output_data)
        }
        // deploy Group("a1", "a2"), naming "group_a1_a2"
        {
            let output_data = OutPointVec::new_builder()
                .set(vec![
                    deployer.get_out_point("a1"),
                    deployer.get_out_point("a2"),
                ])
                .build()
                .as_bytes()
                .pack();
            let output = CellOutput::new_builder()
                .lock(node2021.always_success_script())
                .build_exact_capacity(Capacity::bytes(output_data.len()).unwrap())
                .unwrap();
            deployer.deploy(node2021, "group_a1_a2", output, output_data)
        }
        // deploy Group("a1", "b1"), naming "group_a1_b1"
        {
            let output_data = OutPointVec::new_builder()
                .set(vec![
                    deployer.get_out_point("a1"),
                    deployer.get_out_point("b1"),
                ])
                .build()
                .as_bytes()
                .pack();
            let output = CellOutput::new_builder()
                .lock(node2021.always_success_script())
                .build_exact_capacity(Capacity::bytes(output_data.len()).unwrap())
                .unwrap();
            deployer.deploy(node2021, "group_a1_b1", output, output_data)
        }

        let code_hash_via_data_hash = {
            let out_point = deployer.get_out_point("a1");
            let cell_with_status = node2021.rpc_client().get_live_cell(out_point.into(), true);
            let raw_data = cell_with_status.cell.unwrap().data.unwrap().content;
            CellOutput::calc_data_hash(raw_data.as_bytes())
        };
        let code_hash_via_type_hash = { type_script.calc_script_hash() };

        // Assert the current tip is lower than fork switch height
        assert!(node2021.get_tip_block().epoch().number() < RFC0029_EPOCH_NUMBER);

        let input = node2021.get_spendable_always_success_cells()[0].to_owned();
        for case in self.cases_params() {
            let node = {
                let node = node2021.clone_node(&format!("case-{}-node", case.id));
                node.pull_node(node2021).unwrap();
                node
            };
            let tx = self.build_transaction(
                &code_hash_via_data_hash,
                &code_hash_via_type_hash,
                node2021,
                &deployer,
                &input,
                case.script_hash_type,
                case.cell_deps,
            );
            let actual_result = node.build_according_to_instructions(
                case.height,
                vec![
                    BuildInstruction::Propose {
                        template_number: case.height - 2,
                        proposal_short_id: tx.proposal_short_id(),
                    },
                    BuildInstruction::Commit {
                        template_number: case.height,
                        transaction: tx,
                    },
                ],
            );
            assert_result_eq!(
                case.expected_result,
                actual_result,
                "case.id: {}, node.log: {}",
                case.id,
                node.log_path().to_string_lossy()
            );
        }
    }
}

impl RFC0029 {
    fn cases_params(&self) -> Vec<CaseParams> {
        vec![
            CaseParams {
                id: 0,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 1,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["a1", "a1"],
                expected_result: Err(ERROR_DUPLICATE_CELL_DEPS),
            },
            CaseParams {
                id: 2,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["a1", "a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 3,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["a1", "b1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 4,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 5,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1_a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 6,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1_a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 7,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1_b1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 8,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1", "a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 9,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1", "a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 10,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1", "b1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 11,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1", "group_a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 12,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1", "group_b1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 13,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 14,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["a1", "a1"],
                expected_result: Err(ERROR_DUPLICATE_CELL_DEPS),
            },
            CaseParams {
                id: 15,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["a1", "a2"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 16,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["a1", "b1"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 17,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 18,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1_a1"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 19,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1_a2"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 20,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1_b1"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 21,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1", "a1"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 22,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1", "a2"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 23,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1", "b1"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 24,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1", "group_a2"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 25,
                height: RFC0029_BLOCK_NUMBER - 1,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1", "group_b1"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 26,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 27,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["a1", "a1"],
                expected_result: Err(ERROR_DUPLICATE_CELL_DEPS),
            },
            CaseParams {
                id: 28,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["a1", "a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 29,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["a1", "b1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 30,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 31,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1_a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 32,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1_a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 33,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1_b1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 34,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1", "a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 35,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1", "a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 36,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1", "b1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 37,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1", "group_a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 38,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Data,
                cell_deps: vec!["group_a1", "group_b1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 39,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 40,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["a1", "a1"],
                expected_result: Err(ERROR_DUPLICATE_CELL_DEPS),
            },
            CaseParams {
                id: 41,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["a1", "a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 42,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["a1", "b1"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 43,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 44,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1_a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 45,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1_a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 46,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1_b1"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 47,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1", "a1"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 48,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1", "a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 49,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1", "b1"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
            CaseParams {
                id: 50,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1", "group_a2"],
                expected_result: Ok(()),
            },
            CaseParams {
                id: 51,
                height: RFC0029_BLOCK_NUMBER,
                script_hash_type: ScriptHashType::Type,
                cell_deps: vec!["group_a1", "group_b1"],
                expected_result: Err(ERROR_MULTIPLE_MATCHES),
            },
        ]
    }

    fn build_transaction(
        &self,
        code_hash_via_data_hash: &Byte32,
        code_hash_via_type_hash: &Byte32,
        node: &Node,
        deployer: &Deployer,
        input: &CellMeta,
        script_hash_type: ScriptHashType,
        str_cell_deps: Vec<&str>,
    ) -> TransactionView {
        let type_ = {
            let code_hash = match script_hash_type {
                ScriptHashType::Data => code_hash_via_data_hash.clone(),
                ScriptHashType::Type => code_hash_via_type_hash.clone(),
                ScriptHashType::Data1 => unreachable!(),
            };
            Script::new_builder()
                .hash_type(script_hash_type.into())
                .code_hash(code_hash)
                .build()
        };
        let output = CellOutput::new_builder()
            .lock(node.always_success_script())
            .type_(Some(type_).pack())
            .build_exact_capacity(Capacity::zero())
            .unwrap();
        let cell_deps = {
            let mut cell_deps = Vec::new();
            // cell-deps for output.lock
            cell_deps.push(node.always_success_cell_dep());
            // cell-deps for output.type_
            for cell_name in str_cell_deps {
                let cell_meta = deployer.get_cell(cell_name);
                let dep_type = if cell_name.contains("group") {
                    DepType::DepGroup
                } else {
                    DepType::Code
                };
                let cell_dep = CellDep::new_builder()
                    .dep_type(dep_type.into())
                    .out_point(cell_meta.out_point)
                    .build();
                cell_deps.push(cell_dep);
            }
            cell_deps
        };
        TransactionBuilder::default()
            .input(CellInput::new(input.out_point.clone(), 0))
            .output(output)
            .output_data(Default::default())
            .cell_deps(cell_deps)
            .build()
    }
}
