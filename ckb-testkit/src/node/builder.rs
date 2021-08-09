use crate::{Node, NodeOptions};
use ckb_jsonrpc_types::TransactionTemplate;
use ckb_types::{
    core::{BlockNumber, TransactionView},
    packed::{self, ProposalShortId},
    prelude::*,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum BuildInstruction {
    SendTransaction {
        block_number: BlockNumber,
        transaction: TransactionView,
    },
    Propose {
        block_number: BlockNumber,
        proposal_short_id: ProposalShortId,
    },
    Commit {
        block_number: BlockNumber,
        transaction: TransactionView,
    },
    ProcessWithoutVerify {
        block_number: BlockNumber,
    },
}

impl BuildInstruction {
    pub fn block_number(&self) -> BlockNumber {
        match self {
            BuildInstruction::SendTransaction { block_number, .. } => *block_number,
            BuildInstruction::Propose { block_number, .. } => *block_number,
            BuildInstruction::Commit { block_number, .. } => *block_number,
            BuildInstruction::ProcessWithoutVerify { block_number } => *block_number,
        }
    }
}

impl Node {
    pub fn build_according_to_instructions(
        &self,
        target_height: BlockNumber,
        instructions: Vec<BuildInstruction>,
    ) -> Result<(), String>{
        let initial_tip_number = self.get_tip_block_number();
        assert!(self.consensus().permanent_difficulty_in_dummy);
        assert!(instructions.iter().all(|option| {
            initial_tip_number < option.block_number() && option.block_number() <= target_height
        }));
        let mut params_map: HashMap<BlockNumber, Vec<BuildInstruction>> = HashMap::new();
        for param in instructions {
            params_map
                .entry(param.block_number())
                .or_default()
                .push(param);
        }

        // build chain according to params
        let mut next_template_number = self.get_tip_block_number() + 1;
        loop {
            let mut template = self.rpc_client().get_block_template(None, None, None);
            let number = template.number.value();
            if number > target_height {
                break;
            }
            if number != next_template_number {
                // avoid issues cause by tx-pool async update
                continue;
            } else {
                next_template_number += 1;
            }

            if let Some(params) = params_map.remove(&template.number.value()) {
                let mut process_without_verify = false;
                for param in params {
                    match param {
                        BuildInstruction::SendTransaction { transaction, .. } => {
                            self.rpc_client().send_transaction_result(transaction.data().into()).map_err(|err|err.to_string())?;
                        }
                        BuildInstruction::Propose {
                            proposal_short_id, ..
                        } => {
                            let proposal_short_id = proposal_short_id.into();
                            if !template.proposals.contains(&proposal_short_id) {
                                template.proposals.push(proposal_short_id);
                            }
                        }
                        BuildInstruction::Commit { transaction, .. } => {
                            let transaction_template = TransactionTemplate {
                                hash: transaction.hash().unpack(),
                                data: transaction.data().into(),
                                ..Default::default()
                            };
                            if !template.transactions.contains(&transaction_template) {
                                template.transactions.push(transaction_template);
                            }
                        }
                        BuildInstruction::ProcessWithoutVerify { .. } => {
                            process_without_verify = true;
                        }
                    }
                }
                let updated_block: packed::Block = {
                    let dao_field = self.rpc_client().calculate_dao_field(template.clone());
                    template.dao = dao_field.into();
                    template.into()
                };
                if process_without_verify {
                    self.rpc_client()
                        .process_block_without_verify(updated_block.into(), true);
                } else {
                    self.rpc_client()
                        .submit_block("".to_string(), updated_block.into())
                        .map_err(|err|err.to_string())?;
                }
            } else {
                let block: packed::Block = template.into();
                self.rpc_client()
                    .submit_block("".to_string(), block.into())
                    .map_err(|err|err.to_string())?;
            }
        }
        Ok(())
    }

    /// Return the cloned node with `node_name`.
    pub fn clone_node(&self, node_name: &str) -> Node {
        let mut target_node = {
            let node_options = NodeOptions {
                node_name: String::from(node_name),
                ..self.node_options().clone()
            };
            let is_ckb2021 = self.rpc_client().ckb2021;
            Node::init("cloned_node", node_options, is_ckb2021)
        };
        target_node.start();

        target_node.pull_node(self);

        target_node
    }

    pub fn pull_node(&self, source_node: &Node) {
        assert!(self.get_tip_block_number() <= source_node.get_tip_block_number());
        let min_tip_number = self.get_tip_block_number();
        let max_tip_number = source_node.get_tip_block_number();
        let mut fixed_number = min_tip_number;

        for number in (0..=min_tip_number).rev() {
            if self.rpc_client().get_block_hash(number) == source_node.rpc_client().get_block_hash(number) {
                fixed_number = number;
                break;
            }
        }

        for number in fixed_number..=max_tip_number {
            let block = source_node.get_block_by_number(number);
            self.submit_block(&block);
        }
    }
}
