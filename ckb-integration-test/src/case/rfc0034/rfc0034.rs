use super::{ERROR_INVALID_ECALL, RFC0034_EPOCH_NUMBER};
use crate::prelude::*;
use crate::util::deployer::Deployer;
use crate::util::estimate_start_number_of_epoch;
use ckb_exec_params::ExecParams;
use ckb_testkit::assert_result_eq;
use ckb_testkit::ckb_types::{
    core::{Capacity, ScriptHashType, TransactionBuilder, TransactionView},
    packed::{Bytes, CellDep, CellInput, CellOutput, OutPoint, Script},
    prelude::*,
};

/// * `output.type_.code_hash` points to `exec_caller`
/// * `exec`'s parameter `bounds` is always be `0`
/// * `exec`'s parameter `index` is always be `0`
///
/// ```text
/// ┌────────┬───────────┬───────────┬─────────────────────────────┬──────────────┐
/// │ height │   source  │   place   │ transaction                 │  result      │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = null           │              │
/// │ 2999   │   Output  │    Data   │ output.data = exec_callee   │ InvalidEcall │
/// │        │           │           │ witness = null              │              │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = null           │              │
/// │ 2999   │   Output  │  Witness  │ output.data = null          │ InvalidEcall │
/// │        │           │           │ witness = exec_callee       │              │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = exec_callee    │              │
/// │ 2999   │   Input   │    Data   │ output.data = null          │ InvalidEcall │
/// │        │           │           │ witness = null              │              │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = null           │ InvalidEcall │
/// │ 2999   │   Input   │  Witness  │ output.data = null          │              │
/// │        │           │           │ witness = exec_callee       │              │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = null           │              │
/// │ 2999   │  DepCell  │    Data   │ output.data = null          │ InvalidEcall │
/// │        │           │           │ witness = null              │              │
/// │        │           │           │ dep_cell.data = exec_callee │              │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = null           │              │
/// │ 2999   │  DepCell  │  Witness  │ output.data = null          │ InvalidEcall │
/// │        │           │           │ witness = exec_callee       │              │
/// │        │           │           │                             │              │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = null           │              │
/// │ 3000   │   Output  │    Data   │ output.data = exec_callee   │     Pass     │
/// │        │           │           │ witness = null              │              │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = null           │     Pass     │
/// │ 3000   │   Output  │  Witness  │ output.data = null          │              │
/// │        │           │           │ witness = exec_callee       │              │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = exec_callee    │     Pass     │
/// │ 3000   │   Input   │    Data   │ output.data = null          │              │
/// │        │           │           │ witness = null              │              │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = null           │     Pass     │
/// │ 3000   │   Input   │  Witness  │ output.data = null          │              │
/// │        │           │           │ witness = exec_callee       │              │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = null           │              │
/// │ 3000   │  DepCell  │    Data   │ output.data = null          │     Pass     │
/// │        │           │           │ witness = null              │              │
/// │        │           │           │ dep_cell.data = exec_callee │              │
/// ├────────┼───────────┼───────────┼─────────────────────────────┼──────────────┤
/// │        │           │           │ input.data = null           │              │
/// │ 3000   │  DepCell  │  Witness  │ output.data = null          │ OutOfBound   │
/// │        │           │           │ witness = exec_callee       │     &        │
/// │        │           │           │                             │ InvalidEcall │
/// └────────┴───────────┴───────────┴─────────────────────────────┴──────────────┘
/// ```
pub struct RFC0034;

impl Case for RFC0034 {
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

        // Make sure the VM1 is activated
        let fork_switch_height = estimate_start_number_of_epoch(node2021, RFC0034_EPOCH_NUMBER);
        node2021.mine_to(fork_switch_height + 10);

        // Deploy contract cells
        let mut deployer = Deployer::new();
        {
            let scripts_data = vec![
                (
                    "exec_callee",
                    include_bytes!("../../../testdata/script/exec_callee").pack(),
                ),
                (
                    "exec_caller",
                    include_bytes!("../../../testdata/script/exec_caller").pack(),
                ),
            ];
            for (script_name, script_data) in scripts_data {
                let output = CellOutput::new_builder()
                    .lock(node2021.always_success_script())
                    .build_exact_capacity(Capacity::bytes(script_data.len()).unwrap())
                    .unwrap();
                deployer.deploy(node2021, script_name, output, script_data);
            }
        }

        for case in self.cases_params() {
            let txs = self.build_transactions(node2021, &deployer, &case);
            let node = node2021.clone_node(&format!("case-{}", case.id));
            node.pull_node(node2021).unwrap();

            let actual_result = self.run_case(&node, &txs);
            assert_result_eq!(
                case.expected_result,
                actual_result,
                "case: {}, expected: {:?}, actual: {:?}, node.log_path: {}",
                case.id,
                case.expected_result,
                actual_result,
                node.log_path().to_string_lossy(),
            );
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ExecSource {
    Input = 0x0000000000000001,
    Output = 0x0000000000000002,
    CellDep = 0x0000000000000003,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ExecPlace {
    CellData = 0,
    Witness = 1,
}

#[derive(Debug)]
struct CaseParams {
    id: usize,
    exec_source: ExecSource,
    exec_place: ExecPlace,
    script_hash_type: ScriptHashType,
    expected_result: Result<(), String>,
}

impl RFC0034 {
    fn run_case(&self, node: &Node, txs: &[TransactionView]) -> Result<(), String> {
        for tx in txs {
            node.rpc_client()
                .send_transaction_result(tx.data().into())
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    fn build_transactions(
        &self,
        node: &Node,
        deployer: &Deployer,
        case: &CaseParams,
    ) -> Vec<TransactionView> {
        let mut spendable = node.get_spendable_always_success_cells();

        // Prepare common-used utils
        let exec_callee_data: Bytes = {
            let exec_callee_cell = deployer.get_cell("exec_callee");
            let cell_with_status = node
                .rpc_client()
                .get_live_cell(exec_callee_cell.out_point.clone().into(), true);
            let raw_data = cell_with_status.cell.unwrap().data.unwrap().content;
            raw_data.into_bytes().pack()
        };
        let exec_caller_cell_dep = CellDep::new_builder()
            .out_point(deployer.get_cell("exec_caller").out_point.clone())
            .build();
        let exec_caller_output = {
            let exec_params = ExecParams::new_builder()
                .source(ckb_exec_params::ckb_types::prelude::Pack::pack(
                    &(case.exec_source as u32),
                ))
                .place(ckb_exec_params::ckb_types::prelude::Pack::pack(
                    &(case.exec_place as u32),
                ))
                .index(ckb_exec_params::ckb_types::prelude::Pack::pack(&0u32))
                .bounds(ckb_exec_params::ckb_types::prelude::Pack::pack(&0u64))
                .build();
            let exec_caller_data_hash = {
                let exec_caller_out_point = deployer.get_out_point("exec_caller");
                let cell_with_status = node
                    .rpc_client()
                    .get_live_cell(exec_caller_out_point.into(), true);
                let raw_data = cell_with_status.cell.unwrap().data.unwrap().content;
                CellOutput::calc_data_hash(raw_data.as_bytes())
            };
            CellOutput::new_builder()
                .lock(node.always_success_script())
                .type_(
                    Some({
                        assert!(
                            case.script_hash_type == ScriptHashType::Data
                                || case.script_hash_type == ScriptHashType::Data1
                        );
                        Script::new_builder()
                            .hash_type(case.script_hash_type.into())
                            .code_hash(exec_caller_data_hash)
                            // `exec_params.as_slice().pack()` or `exec_params.as_bytes().pack()`?
                            .args(exec_params.as_slice().pack())
                            .build()
                    })
                    .pack(),
                )
                .build_exact_capacity(Capacity::bytes(exec_callee_data.len()).unwrap())
                .unwrap()
        };
        let dep_tx = {
            let inputs = spendable.split_off(spendable.len() - 100);
            let capacity: u64 = inputs.iter().map(|input| input.capacity().as_u64()).sum();
            TransactionBuilder::default()
                .inputs(
                    inputs
                        .iter()
                        .map(|input| CellInput::new(input.out_point.clone(), 0)),
                )
                .output(
                    CellOutput::new_builder()
                        .lock(inputs[0].cell_output.lock())
                        .type_(inputs[0].cell_output.type_())
                        .capacity(capacity.pack())
                        .build(),
                )
                .output_data(exec_callee_data.clone())
                .cell_dep(node.always_success_cell_dep())
                .build()
        };

        if case.exec_source == ExecSource::Input && case.exec_place == ExecPlace::CellData {
            let tx = TransactionBuilder::default()
                .input(CellInput::new(OutPoint::new(dep_tx.hash(), 0), 0))
                .output(exec_caller_output)
                .output_data(Default::default())
                .cell_dep(exec_caller_cell_dep.clone())
                .cell_dep(node.always_success_cell_dep())
                .build();
            return vec![dep_tx, tx];
        }
        if case.exec_source == ExecSource::CellDep && case.exec_place == ExecPlace::CellData {
            let inputs = spendable.split_off(spendable.len() - 100);
            let tx = TransactionBuilder::default()
                .inputs(
                    inputs
                        .into_iter()
                        .map(|input| CellInput::new(input.out_point, 0)),
                )
                .output(exec_caller_output)
                .output_data(Default::default())
                .cell_dep(
                    CellDep::new_builder()
                        .out_point(OutPoint::new(dep_tx.hash(), 0))
                        .build(),
                )
                .cell_dep(exec_caller_cell_dep.clone())
                .cell_dep(node.always_success_cell_dep())
                .build();
            return vec![dep_tx, tx];
        }

        let mut tx_builder = TransactionBuilder::default();

        // build tx's input
        // build tx's output
        // build tx's cell-dep
        let inputs = spendable.split_off(spendable.len() - 100);
        tx_builder = tx_builder
            .inputs(
                inputs
                    .into_iter()
                    .map(|input| CellInput::new(input.out_point, 0)),
            )
            .output(exec_caller_output)
            .cell_dep(exec_caller_cell_dep.clone())
            .cell_dep(node.always_success_cell_dep());

        // build tx's output-data
        if case.exec_place == ExecPlace::CellData {
            tx_builder = tx_builder.output_data(exec_callee_data.clone());
        } else {
            tx_builder = tx_builder.output_data(Default::default())
        }

        // build tx's witness
        if case.exec_place == ExecPlace::Witness {
            tx_builder = tx_builder.witness(exec_callee_data);
        }

        let tx = tx_builder.build();
        vec![dep_tx, tx]
    }

    fn cases_params(&self) -> Vec<CaseParams> {
        vec![
            CaseParams {
                id: 0,
                exec_source: ExecSource::Input,
                exec_place: ExecPlace::CellData,
                script_hash_type: ScriptHashType::Data,
                expected_result: Err(ERROR_INVALID_ECALL.to_string()),
            },
            CaseParams {
                id: 1,
                exec_source: ExecSource::Input,
                exec_place: ExecPlace::Witness,
                script_hash_type: ScriptHashType::Data,
                expected_result: Err(ERROR_INVALID_ECALL.to_string()),
            },
            CaseParams {
                id: 2,
                exec_source: ExecSource::Output,
                exec_place: ExecPlace::CellData,
                script_hash_type: ScriptHashType::Data,
                expected_result: Err(ERROR_INVALID_ECALL.to_string()),
            },
            CaseParams {
                id: 3,
                exec_source: ExecSource::Output,
                exec_place: ExecPlace::Witness,
                script_hash_type: ScriptHashType::Data,
                expected_result: Err(ERROR_INVALID_ECALL.to_string()),
            },
            CaseParams {
                id: 4,
                exec_source: ExecSource::CellDep,
                exec_place: ExecPlace::CellData,
                script_hash_type: ScriptHashType::Data,
                expected_result: Err(ERROR_INVALID_ECALL.to_string()),
            },
            CaseParams {
                id: 5,
                exec_source: ExecSource::CellDep,
                exec_place: ExecPlace::Witness,
                script_hash_type: ScriptHashType::Data,
                expected_result: Err(ERROR_INVALID_ECALL.to_string()),
            },
            CaseParams {
                id: 6,
                exec_source: ExecSource::Input,
                exec_place: ExecPlace::CellData,
                script_hash_type: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 7,
                exec_source: ExecSource::Input,
                exec_place: ExecPlace::Witness,
                script_hash_type: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 8,
                exec_source: ExecSource::Output,
                exec_place: ExecPlace::CellData,
                script_hash_type: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 9,
                exec_source: ExecSource::Output,
                exec_place: ExecPlace::Witness,
                script_hash_type: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 10,
                exec_source: ExecSource::CellDep,
                exec_place: ExecPlace::CellData,
                script_hash_type: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 11,
                exec_source: ExecSource::CellDep,
                exec_place: ExecPlace::Witness,
                script_hash_type: ScriptHashType::Data1,
                expected_result: Err("TransactionScriptError".to_string()),
            },
        ]
    }
}
