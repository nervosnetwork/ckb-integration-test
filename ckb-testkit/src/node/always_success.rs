use crate::Node;
use ckb_types::{
    core::{cell::CellMeta, ScriptHashType, TransactionBuilder, TransactionView},
    packed::{CellDep, CellInput, CellOutput, OutPoint, Script},
    prelude::*,
};

pub const SYSTEM_CELL_ALWAYS_SUCCESS_INDEX: u32 = 5;

impl Node {
    pub fn always_success_script(&self) -> Script {
        let genesis_cellbase_hash = self.genesis_cellbase_hash();
        let always_success_out_point =
            OutPoint::new(genesis_cellbase_hash, SYSTEM_CELL_ALWAYS_SUCCESS_INDEX);
        let cell = self
            .rpc_client()
            .get_live_cell(always_success_out_point.into(), false);
        let cell_info = cell.cell.expect("genesis always cell must be live");
        let cell_output: CellOutput = cell_info.output.into();
        let type_ = cell_output
            .type_()
            .to_opt()
            .expect("genesis always success cell should have type_=type-id script");
        Script::new_builder()
            .code_hash(type_.calc_script_hash())
            .hash_type(ScriptHashType::Type.into())
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

    pub fn get_spendable_always_success_cells(&self) -> Vec<CellMeta> {
        let live_out_points = self
            .indexer()
            .get_live_cells_by_lock_script(&self.always_success_script())
            .expect("indexer get_live_cells_by_lock_script");
        live_out_points
            .into_iter()
            .filter_map(|out_point| {
                let cell_meta = self.get_cell_meta(out_point);
                if cell_meta.data_bytes == 0 {
                    Some(cell_meta)
                } else {
                    None
                }
            })
            .collect()
    }
}
