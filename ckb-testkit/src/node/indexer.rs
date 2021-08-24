use crate::Node;
use ckb_types::core::cell::{CellMeta, CellMetaBuilder};
use ckb_types::core::{BlockView, EpochNumberWithFraction, TransactionInfo};
use ckb_types::packed::OutPoint;

impl Node {
    // NOTICE: This function use `indexer_unchecked`
    pub fn get_cell_meta(&self, out_point: OutPoint) -> Option<CellMeta> {
        let detail = self
            .indexer_unchecked()
            .get_detailed_live_cell(&out_point)
            .expect("indexer get_detailed_live_cell")?;
        // FIXME now the transaction_info.block_epoch is fake
        // let block_epoch = self.get_block(detail.block_hash.clone()).epoch();
        let block_epoch = EpochNumberWithFraction::new_unchecked(0, 0, 0);
        let txinfo = TransactionInfo::new(
            detail.block_number,
            block_epoch,
            detail.block_hash,
            detail.tx_index as usize,
        );
        Some(
            CellMetaBuilder::from_cell_output(detail.cell_output, detail.cell_data.raw_data())
                .out_point(out_point)
                .transaction_info(txinfo)
                .build(),
        )
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
