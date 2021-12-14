use crate::preclude::*;
use ckb_testkit::ckb_types::{
    core::{BlockNumber, Capacity, Cycle, ScriptHashType, TransactionBuilder, TransactionView},
    packed::{CellInput, CellOutput, OutPoint, Script},
    prelude::*,
};
use ckb_testkit::{assert_result_eq, Node, NodeOptions, Nodes, SYSTEM_CELL_ALWAYS_SUCCESS_INDEX};

/// ## Convention
///
/// 1. Fork2021 activates at height `3000`.
///
/// ## Note
///
/// * We determine the VM selection via checking the transaction cycles.
/// * We want the input transaction is VM-determined, so in this case,
/// we config node with `app_config: "testdata/config/ckb2021_block_assembler_hash_type_is_data/ckb.toml"`
///
/// ## Cases
///
/// ```text
/// ┌───┬────────────────┬────────────────┬───────────────────┐
/// │id │ type.hash_type │ height         │ selected          │
/// ├───┼────────────────┼────────────────┼───────────────────┤
/// │0  │ "data"         │  non-activated │ Ok(VM0)           │
/// ├───┼────────────────┼────────────────┼───────────────────┤
/// │1  │ "type"         │  non-activated │ Ok(VM0)           │
/// ├───┼────────────────┼────────────────┼───────────────────┤
/// │2  │ "data1"        │  non-activated │ Err(Incompatible) │
/// ├───┼────────────────┼────────────────┼───────────────────┤
/// │3  │ "data"         │  activated     │ Ok(VM0)           │
/// ├───┼────────────────┼────────────────┼───────────────────┤
/// │4  │ "type"         │  activated     │ Ok(VM1)           │
/// ├───┼────────────────┼────────────────┼───────────────────┤
/// │5  │ "data1"        │  activated     │ Ok(VM1)           │
/// └───┴────────────────┴────────────────┴───────────────────┘
/// ```

pub struct RFC0032;

const VM0_CYCLES: Cycle = 537 + 537;
const VM1_CYCLES: Cycle = 537 + 539;
const RFC0032_BLOCK_NUMBER: BlockNumber = 3000;
const HARDFORK_DELAY_WINDOW: u64 = 10;
const ERROR_INVALID_VM_VERSION: &str = "Invalid VM Version";

impl Case for RFC0032 {
    fn case_options(&self) -> CaseOptions {
        Default::default()
    }

    fn run(&self, _nodes: Nodes) {
        for case in self.cases_params(RFC0032_BLOCK_NUMBER, VM0_CYCLES, VM1_CYCLES) {
            let node = self.setup_node(&case);
            let tx = self.build_transaction(&node, &case);
            let actual_result = self.run_case(&node, &tx);
            assert_result_eq!(
                case.expected_result,
                actual_result,
                "case.id: {}, node.log_path: {}, tx: {:#x}",
                case.id,
                node.log_path().to_string_lossy(),
                tx.hash(),
            );
        }
    }
}

#[derive(Debug)]
struct CaseParams {
    id: usize,
    type_script_hash_type: ScriptHashType,
    height: BlockNumber,
    expected_result: Result<Cycle, String>,
}

impl RFC0032 {
    fn run_case(&self, node: &Node, transaction: &TransactionView) -> Result<Cycle, String> {
        let old_tx_pool_total_cycles = node.get_tip_tx_pool_info().total_tx_cycles;
        let _tx_hash = node
            .rpc_client()
            .send_transaction_result(transaction.data().into())
            .map_err(|err| err.to_string())?;
        let new_tx_pool_total_cycles = node.get_tip_tx_pool_info().total_tx_cycles;
        Ok(new_tx_pool_total_cycles.value() - old_tx_pool_total_cycles.value())
    }

    fn setup_node(&self, case: &CaseParams) -> Node {
        let node_options = NodeOptions {
            node_name: format!("{}-case-{}", self.case_name(), case.id),
            ckb_binary: CKB2021.read().unwrap().clone(),
            initial_database: "testdata/db/Epoch2V2TestData",
            chain_spec: "testdata/spec/ckb2021",
            // We want the input transaction is VM-determined
            app_config: "testdata/config/ckb2021_block_assembler_hash_type_is_data",
        };
        let mut node = Node::init(self.case_name(), node_options, true);
        node.start();

        node.mine_to(case.height);

        node
    }

    fn build_transaction(&self, node: &Node, case: &CaseParams) -> TransactionView {
        let input = {
            let tip_block = node.get_tip_block();
            let tip_cellbase = tip_block.transaction(0).unwrap();
            node.indexer();
            node.get_cell_meta(OutPoint::new(tip_cellbase.hash(), 0))
                .unwrap()
        };
        assert_eq!(
            input.cell_output.lock().hash_type(),
            ScriptHashType::Data.into()
        );
        let type_ = {
            match case.type_script_hash_type {
                ScriptHashType::Data | ScriptHashType::Data1 => {
                    let always_script_data_hash = {
                        let genesis_cellbase_hash = node.genesis_cellbase_hash();
                        let always_success_out_point =
                            OutPoint::new(genesis_cellbase_hash, SYSTEM_CELL_ALWAYS_SUCCESS_INDEX);
                        let cell = node
                            .rpc_client()
                            .get_live_cell(always_success_out_point.into(), true);
                        let cell_info = cell.cell.expect("genesis always cell must be live");
                        let cell_data_hash = cell_info.data.unwrap().hash;
                        cell_data_hash.pack()
                    };
                    Script::new_builder()
                        .hash_type(case.type_script_hash_type.into())
                        .code_hash(always_script_data_hash)
                        .build()
                }
                ScriptHashType::Type => {
                    let always_script_type_hash = {
                        let script = node.always_success_script();
                        assert!(script.hash_type() == ScriptHashType::Type.into());
                        script.code_hash()
                    };
                    Script::new_builder()
                        .hash_type(case.type_script_hash_type.into())
                        .code_hash(always_script_type_hash)
                        .build()
                }
            }
        };
        TransactionBuilder::default()
            .input(CellInput::new(input.out_point.clone(), 0))
            .output(
                CellOutput::new_builder()
                    .lock(input.cell_output.lock())
                    .type_(Some(type_).pack())
                    .build_exact_capacity(Capacity::zero())
                    .unwrap(),
            )
            .output_data(Default::default())
            .cell_dep(node.always_success_cell_dep())
            .build()
    }

    fn cases_params(
        &self,
        rfc0032_block_number: BlockNumber,
        vm0_tx_cycles: Cycle,
        vm1_tx_cycles: Cycle,
    ) -> Vec<CaseParams> {
        vec![
            CaseParams {
                id: 0,
                type_script_hash_type: ScriptHashType::Data,
                height: rfc0032_block_number - HARDFORK_DELAY_WINDOW - 1,
                expected_result: Ok(vm0_tx_cycles),
            },
            CaseParams {
                id: 1,
                type_script_hash_type: ScriptHashType::Type,
                height: rfc0032_block_number - HARDFORK_DELAY_WINDOW - 1,
                expected_result: Ok(vm0_tx_cycles),
            },
            CaseParams {
                id: 2,
                type_script_hash_type: ScriptHashType::Data1,
                height: rfc0032_block_number - HARDFORK_DELAY_WINDOW - 1,
                expected_result: Err(ERROR_INVALID_VM_VERSION.to_string()),
            },
            CaseParams {
                id: 3,
                type_script_hash_type: ScriptHashType::Data,
                height: rfc0032_block_number + HARDFORK_DELAY_WINDOW + 1,
                expected_result: Ok(vm0_tx_cycles),
            },
            CaseParams {
                id: 4,
                type_script_hash_type: ScriptHashType::Type,
                height: rfc0032_block_number + HARDFORK_DELAY_WINDOW + 1,
                expected_result: Ok(vm1_tx_cycles),
            },
            CaseParams {
                id: 5,
                type_script_hash_type: ScriptHashType::Data1,
                height: rfc0032_block_number + HARDFORK_DELAY_WINDOW + 1,
                expected_result: Ok(vm1_tx_cycles),
            },
        ]
    }
}
