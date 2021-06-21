use crate::node::Node;
use ckb_types::packed::CellInput;
use ckb_types::{
    bytes,
    core::{cell::CellMeta, ScriptHashType, TransactionBuilder, TransactionView},
    packed::{CellDep, CellOutput, OutPoint, Script},
    prelude::*,
};

pub const SYSTEM_CELL_ALWAYS_SUCCESS_INDEX: u32 = 5;

impl Node {
    pub fn always_success_raw_data(&self) -> bytes::Bytes {
        self.genesis_block().transactions()[0]
            .outputs_data()
            .get(SYSTEM_CELL_ALWAYS_SUCCESS_INDEX as usize)
            .unwrap()
            .raw_data()
    }

    pub fn always_success_script(&self) -> Script {
        let always_success_raw = self.always_success_raw_data();
        let always_success_code_hash = CellOutput::calc_data_hash(&always_success_raw);
        Script::new_builder()
            .code_hash(always_success_code_hash)
            .hash_type(ScriptHashType::Data.into())
            .build()
    }

    pub fn always_success_cell_dep(&self) -> CellDep {
        let genesis_cellbase_hash = self.genesis_cellbase_hash();
        let always_success_out_point =
            OutPoint::new(genesis_cellbase_hash, SYSTEM_CELL_ALWAYS_SUCCESS_INDEX);
        CellDep::new_builder()
            .out_point(always_success_out_point)
            .build()
    }

    pub fn always_success_transaction(&self, cell: &CellMeta) -> TransactionView {
        TransactionBuilder::default()
            .input(CellInput::new(cell.out_point.clone(), 0))
            .output(
                CellOutput::new_builder()
                    .lock(cell.cell_output.lock())
                    .type_(cell.cell_output.type_())
                    .capacity(cell.capacity().pack())
                    .build(),
            )
            .output_data(Default::default())
            .cell_dep(self.always_success_cell_dep())
            .build()
    }
}
