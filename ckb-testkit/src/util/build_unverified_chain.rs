use crate::util::wait_until;
use crate::{Node, NodeOptions};
use ckb_types::core::BlockView;
use ckb_types::{
    core::{BlockNumber, TransactionView},
    packed::{self, ProposalShortId},
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum BuildUnverifiedChainParam {
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

impl BuildUnverifiedChainParam {
    pub fn block_number(&self) -> BlockNumber {
        match self {
            BuildUnverifiedChainParam::Pending { block_number, .. } => *block_number,
            BuildUnverifiedChainParam::Proposal { block_number, .. } => *block_number,
            BuildUnverifiedChainParam::Committed { block_number, .. } => *block_number,
        }
    }
}

pub fn build_unverified_chain(
    base_node: &Node,
    target_height: BlockNumber,
    params: Vec<BuildUnverifiedChainParam>,
) -> Vec<BlockView> {
    let base_tip_number = base_node.get_tip_block_number();
    assert!(base_node.consensus().permanent_difficulty_in_dummy);
    assert!(params.iter().all(|option| {
        base_tip_number < option.block_number() && option.block_number() <= target_height
    }));
    let mut params_map: HashMap<BlockNumber, Vec<BuildUnverifiedChainParam>> = HashMap::new();
    for param in params {
        params_map
            .entry(param.block_number())
            .or_default()
            .push(param);
    }

    // Create a helper node. Lately use it to construct chain
    let mut helper_node = {
        let node_options = NodeOptions {
            node_name: "helper",
            ..base_node.node_options().clone()
        };
        let is_ckb2021 = base_node.rpc_client().ckb2021;
        Node::init("helper", node_options, is_ckb2021)
    };
    helper_node.start();

    // Sync base chain
    helper_node.p2p_connect(base_node);
    let synced = wait_until(3 * 60, || {
        helper_node.get_tip_block_number() == base_tip_number
    });
    assert!(
        synced,
        "helper_node should sync from {}",
        base_node.node_name()
    );
    helper_node.p2p_disconnect(base_node);

    // build chain according to params
    loop {
        let template = helper_node
            .rpc_client()
            .get_block_template(None, None, None);
        let block = packed::Block::from(template).into_view();
        if block.number() > target_height {
            break;
        }

        if let Some(params) = params_map.remove(&block.number()) {
            let mut block_builder = block.as_advanced_builder();
            for param in params {
                match param {
                    BuildUnverifiedChainParam::Pending { transaction, .. } => {
                        helper_node.submit_transaction(&transaction);
                    }
                    BuildUnverifiedChainParam::Proposal {
                        proposal_short_id, ..
                    } => {
                        block_builder = block_builder.proposal(proposal_short_id);
                    }
                    BuildUnverifiedChainParam::Committed { transaction, .. } => {
                        block_builder = block_builder.transaction(transaction);
                    }
                }
            }
            let block = block_builder.build();
            helper_node
                .rpc_client()
                .process_block_without_verify(block.data().into(), false);
        } else {
            helper_node
                .rpc_client()
                .process_block_without_verify(block.data().into(), false);
        }
    }

    (base_tip_number..=helper_node.get_tip_block_number())
        .map(|number| helper_node.get_block_by_number(number))
        .collect()
}
