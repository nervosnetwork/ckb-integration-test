use crate::Node;
use ckb_types::packed;

impl Node {
    pub fn mine(&self, n_blocks: u64) {
        for _ in 0..n_blocks {
            let template = self.rpc_client().get_block_template(None, None, None);
            let block = packed::Block::from(template).into_view();
            self.submit_block(&block);
        }
    }
}
