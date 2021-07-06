use crate::node::Node;
use ckb_types::core::cell::{CellMeta, CellMetaBuilder};
use ckb_types::core::{BlockView, TransactionInfo};

impl Node {
    pub fn get_live_always_success_cells(&self) -> Vec<CellMeta> {
        let live_out_points = self
            .indexer()
            .get_live_cells_by_lock_script(&self.always_success_script())
            .expect("indexer get_live_cells_by_lock_script");
        let mut cell_metas = Vec::new();
        for out_point in live_out_points {
            let detail = self
                .indexer()
                .get_detailed_live_cell(&out_point)
                .expect("indexer get_detailed_live_cell")
                .expect("indexer should have detail for live cells");
            let block_epoch = self.get_block(detail.block_hash.clone()).epoch();
            let txinfo = TransactionInfo::new(
                detail.block_number,
                block_epoch,
                detail.block_hash,
                detail.tx_index as usize,
            );
            let cell_meta =
                CellMetaBuilder::from_cell_output(detail.cell_output, detail.cell_data.raw_data())
                    .out_point(out_point)
                    .transaction_info(txinfo)
                    .build();
            cell_metas.push(cell_meta);
        }
        cell_metas
    }

    pub(super) fn wait_for_indexer_synced(&self) {
        let indexer = self.indexer.as_ref().expect("uninitialized indexer");
        loop {
            if let Some((tip_number, tip_hash)) = indexer.tip().expect("indexer tip") {
                let block_opt = self.rpc_client().get_block_by_number(tip_number + 1);
                if let Some(block) = block_opt {
                    let block: BlockView = block.into();
                    if block.parent_hash() != tip_hash {
                        indexer.rollback().expect("indexer rollback")
                    } else {
                        indexer.append(&block).expect("indexer append");
                    }
                } else {
                    let block_hash_opt = self.rpc_client().get_block_hash(tip_number);
                    if block_hash_opt != Some(tip_hash) {
                        indexer.rollback().expect("indexer rollback");
                    } else {
                        break;
                    }
                }
            } else {
                let block = self
                    .rpc_client()
                    .get_block_by_number(0)
                    .expect("rpc get genesis block");
                indexer
                    .append(&block.into())
                    .expect("indexer append genesis block");
            }
        }
    }
}