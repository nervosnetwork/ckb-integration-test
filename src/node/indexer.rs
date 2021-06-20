use crate::node::Node;
use ckb_types::core::BlockView;

impl Node {
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
