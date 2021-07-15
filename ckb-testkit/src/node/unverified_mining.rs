use crate::Node;
use ckb_types::{
    core::{BlockNumber, TransactionView},
    packed::{self, ProposalShortId},
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum UnverifiedMiningOption {
    Pending {
        block_number: BlockNumber,
        transaction: TransactionView,
    },
    Proposal {
        block_number: BlockNumber,
        proposal_short_id: ProposalShortId,
    },
    Committed {
        block_number: BlockNumber,
        transaction: TransactionView,
    },
}

impl UnverifiedMiningOption {
    fn block_number(&self) -> BlockNumber {
        match self {
            UnverifiedMiningOption::Pending { block_number, .. } => *block_number,
            UnverifiedMiningOption::Proposal { block_number, .. } => *block_number,
            UnverifiedMiningOption::Committed { block_number, .. } => *block_number,
        }
    }
}

impl Node {
    pub fn unverified_mine(
        &self,
        target_height: BlockNumber,
        options: Vec<UnverifiedMiningOption>,
    ) {
        assert!(options.iter().all(|option| {
            self.get_tip_block_number() < option.block_number()
                && option.block_number() <= target_height
        }));

        let mut options_map: HashMap<BlockNumber, Vec<UnverifiedMiningOption>> = HashMap::new();
        for option in options {
            options_map
                .entry(option.block_number())
                .or_default()
                .push(option);
        }

        loop {
            let template = self.rpc_client().get_block_template(None, None, None);
            let block = packed::Block::from(template).into_view();
            if block.number() > target_height {
                break;
            }

            if let Some(options) = options_map.remove(&block.number()) {
                let mut block_builder = block.as_advanced_builder();
                for option in options {
                    match option {
                        UnverifiedMiningOption::Pending { transaction, .. } => {
                            self.submit_transaction(&transaction);
                        }
                        UnverifiedMiningOption::Proposal {
                            proposal_short_id, ..
                        } => {
                            block_builder = block_builder.proposal(proposal_short_id);
                        }
                        UnverifiedMiningOption::Committed { transaction, .. } => {
                            block_builder = block_builder.transaction(transaction);
                        }
                    }
                }
                let block = block_builder.build();
                self.rpc_client()
                    .process_block_without_verify(block.data().into(), false);
            } else {
                self.rpc_client()
                    .process_block_without_verify(block.data().into(), false);
            }
        }
    }
}
