use super::{ERROR_DUPLICATE_CELL_DEPS, ERROR_MULTIPLE_MATCHES, RFC0029_EPOCH_NUMBER};
use crate::preclude::*;
use crate::util::deployer::Deployer;
use crate::util::estimate_start_number_of_epoch;
use crate::util::run_case_helper::{run_case_after_switch, run_case_before_switch};
use ckb_testkit::ckb_types::{
    core::{
        cell::CellMeta, Capacity, DepType, ScriptHashType, TransactionBuilder, TransactionView,
    },
    packed::{Byte32, CellDep, CellInput, CellOutput, OutPointVec, Script},
    prelude::*,
};

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
/// │    id  │   hash_type│    cell_deps                       │  2019                  │ 2019                  │
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
        let fork_switch_height = estimate_start_number_of_epoch(node2021, RFC0029_EPOCH_NUMBER);

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

        // [(
        //    case_id,
        //    script.hash_type,
        //    cell_deps,
        //    expected_result_before_switch,
        //    expected_result_after_switch
        // )]
        let cases = vec![
            (0, ScriptHashType::Data, vec!["a1"], Ok(()), Ok(())),
            (
                1,
                ScriptHashType::Data,
                vec!["a1", "a1"],
                Err(ERROR_DUPLICATE_CELL_DEPS),
                Err(ERROR_DUPLICATE_CELL_DEPS),
            ),
            (2, ScriptHashType::Data, vec!["a1", "a2"], Ok(()), Ok(())),
            (3, ScriptHashType::Data, vec!["a1", "b1"], Ok(()), Ok(())),
            (4, ScriptHashType::Data, vec!["group_a1"], Ok(()), Ok(())),
            (5, ScriptHashType::Data, vec!["group_a1_a1"], Ok(()), Ok(())),
            (6, ScriptHashType::Data, vec!["group_a1_a2"], Ok(()), Ok(())),
            (7, ScriptHashType::Data, vec!["group_a1_b1"], Ok(()), Ok(())),
            (
                8,
                ScriptHashType::Data,
                vec!["group_a1", "a1"],
                Ok(()),
                Ok(()),
            ),
            (
                9,
                ScriptHashType::Data,
                vec!["group_a1", "a2"],
                Ok(()),
                Ok(()),
            ),
            (
                10,
                ScriptHashType::Data,
                vec!["group_a1", "b1"],
                Ok(()),
                Ok(()),
            ),
            (
                11,
                ScriptHashType::Data,
                vec!["group_a1", "group_a2"],
                Ok(()),
                Ok(()),
            ),
            (
                12,
                ScriptHashType::Data,
                vec!["group_a1", "group_b1"],
                Ok(()),
                Ok(()),
            ),
            (13, ScriptHashType::Type, vec!["a1"], Ok(()), Ok(())),
            (
                14,
                ScriptHashType::Type,
                vec!["a1", "a1"],
                Err(ERROR_DUPLICATE_CELL_DEPS),
                Err(ERROR_DUPLICATE_CELL_DEPS),
            ),
            (
                15,
                ScriptHashType::Type,
                vec!["a1", "a2"],
                Err(ERROR_MULTIPLE_MATCHES),
                Ok(()),
            ),
            (
                16,
                ScriptHashType::Type,
                vec!["a1", "b1"],
                Err(ERROR_MULTIPLE_MATCHES),
                Err(ERROR_MULTIPLE_MATCHES),
            ),
            (17, ScriptHashType::Type, vec!["group_a1"], Ok(()), Ok(())),
            (
                18,
                ScriptHashType::Type,
                vec!["group_a1_a1"],
                Err(ERROR_MULTIPLE_MATCHES),
                Ok(()),
            ),
            (
                19,
                ScriptHashType::Type,
                vec!["group_a1_a2"],
                Err(ERROR_MULTIPLE_MATCHES),
                Ok(()),
            ),
            (
                20,
                ScriptHashType::Type,
                vec!["group_a1_b1"],
                Err(ERROR_MULTIPLE_MATCHES),
                Err(ERROR_MULTIPLE_MATCHES),
            ),
            (
                21,
                ScriptHashType::Type,
                vec!["group_a1", "a1"],
                Err(ERROR_MULTIPLE_MATCHES),
                Ok(()),
            ),
            (
                22,
                ScriptHashType::Type,
                vec!["group_a1", "a2"],
                Err(ERROR_MULTIPLE_MATCHES),
                Ok(()),
            ),
            (
                23,
                ScriptHashType::Type,
                vec!["group_a1", "b1"],
                Err(ERROR_MULTIPLE_MATCHES),
                Err(ERROR_MULTIPLE_MATCHES),
            ),
            (
                24,
                ScriptHashType::Type,
                vec!["group_a1", "group_a2"],
                Err(ERROR_MULTIPLE_MATCHES),
                Ok(()),
            ),
            (
                25,
                ScriptHashType::Type,
                vec!["group_a1", "group_b1"],
                Err(ERROR_MULTIPLE_MATCHES),
                Err(ERROR_MULTIPLE_MATCHES),
            ),
        ];
        let input = node2021.get_spendable_always_success_cells()[0].to_owned();
        for (
            case_id,
            script_hash_type,
            cell_deps,
            expected_result_before_switch,
            expected_result_after_switch,
        ) in cases
        {
            let tx = build_transaction(
                &code_hash_via_data_hash,
                &code_hash_via_type_hash,
                node2021,
                &deployer,
                &input,
                script_hash_type,
                cell_deps,
            );

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
                vec![tx.clone()],
                expected_result_after_switch,
            );
        }
    }
}

fn build_transaction(
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
