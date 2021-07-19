use crate::util::wait_until;
use crate::{Node, NodeOptions};
use ckb_jsonrpc_types::TransactionTemplate;
use ckb_types::{
    core::{BlockNumber, BlockView, TransactionView},
    packed::{self, ProposalShortId},
    prelude::*,
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

// TODO 建议直接在参数的 node 上操作，不要额外再新建个 node。或者多个 function。
// TODO 加 debug 日志
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
        let mut template = helper_node
            .rpc_client()
            .get_block_template(None, None, None);
        if template.number.value() > target_height {
            break;
        }

        if let Some(params) = params_map.remove(&template.number.value()) {
            for param in params {
                match param {
                    BuildUnverifiedChainParam::Pending { transaction, .. } => {
                        helper_node.submit_transaction(&transaction);
                    }
                    BuildUnverifiedChainParam::Proposal {
                        proposal_short_id, ..
                    } => {
                        template.proposals.push(proposal_short_id.into());
                    }
                    BuildUnverifiedChainParam::Committed { transaction, .. } => {
                        let transaction_template = TransactionTemplate {
                            hash: transaction.hash().unpack(),
                            data: transaction.data().into(),
                            ..Default::default()
                        };
                        template.transactions.push(transaction_template);
                    }
                }
            }
            let dao_field = helper_node
                .rpc_client()
                .calculate_dao_field(template.clone());
            template.dao = dao_field.into();
            let block: packed::Block = template.into();
            helper_node
                .rpc_client()
                .process_block_without_verify(block.into(), false);
        } else {
            let block = packed::Block::from(template).into_view();
            helper_node
                .rpc_client()
                .process_block_without_verify(block.data().into(), false);
        }
    }

    (base_tip_number..=helper_node.get_tip_block_number())
        .map(|number| helper_node.get_block_by_number(number))
        .collect()
}
