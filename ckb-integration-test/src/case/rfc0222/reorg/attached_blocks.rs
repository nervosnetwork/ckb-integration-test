// Rfc0222 allows multiple cell dep matches on type script hash when all the matches are resolved
// to the same script code. **This is a looser change**.
//
// ## Attached Block Transactions
//
// |                      | valid for 2021 only | valid for both 2019 and 2021 |
// | :-----               | ----:               | :----:                       |
// | attach before switch | invalid             | ok                           |
// | attach after switch  | ok                  | ok                           |
//
// - tx_valid_2021_only_attach_before_switch
// - tx_valid_2021_attach_after_switch
// - tx_valid_both_attach_before_switch
// - tx_valid_both_attach_after_switch
//
// Only the case for `tx_valid_2021_only_attach_before_switch` will fail, others will success.
//
// ## Case Description
//
// ```
// chain_a: [2998] [2999] ..(rfc0222 switch).. [3000] [3001]
// chain_b: [2998] [2999] ..(rfc0222 switch).. [3000] [3001] [3002]
// ```
//
// 1. `node` runs chain-a, height is 3001
// 2. Construct a chain-b, height is 3002
//   - case1: chain_b[2999] commits `tx_valid_2021_only_attach_before_switch`
//   - case2: chain_b[3000] commits `tx_valid_2021_attach_after_switch`
//   - case3: chain_b[2999] commits `tx_valid_both_attach_before_switch`
//   - case4: chain_b[3000] commits `tx_valid_both_attach_after_switch`
// 3. Send chain-b to `node`, `node` will trigger reorg
// 4. Check the tip: case1's tip is chain-a[3001], other cases' tip are chain-b[3002]

use crate::case::rfc0222::util::{build_transaction, RFC0222CellDeployer};
use crate::case::{Case, CaseOptions};
use crate::util::calc_epoch_start_number;
use crate::CKB2021;
use ckb_testkit::util::{build_unverified_chain, BuildUnverifiedChainParam};
use ckb_testkit::{Node, NodeOptions, Nodes};
use ckb_types::{
    core::{BlockView, EpochNumber, ScriptHashType},
    packed::Script,
    prelude::*,
};

const RFC0222_EPOCH_NUMBER: EpochNumber = 3;
const ERROR_MULTIPLE_MATCHES: &str = "MultipleMatches";

pub struct RFC0222ReorgAttachedBlocks;

impl Case for RFC0222ReorgAttachedBlocks {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![
                NodeOptions {
                    node_name: "node2021",
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "testdata/db/Epoch2V2TestData",
                    chain_spec: "testdata/spec/ckb2021",
                    app_config: "testdata/config/ckb2021",
                },
                NodeOptions {
                    node_name: "node2021_1",
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "testdata/db/Epoch2V2TestData",
                    chain_spec: "testdata/spec/ckb2021",
                    app_config: "testdata/config/ckb2021",
                },
                NodeOptions {
                    node_name: "node2021_2",
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "testdata/db/Epoch2V2TestData",
                    chain_spec: "testdata/spec/ckb2021",
                    app_config: "testdata/config/ckb2021",
                },
                NodeOptions {
                    node_name: "node2021_3",
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "testdata/db/Epoch2V2TestData",
                    chain_spec: "testdata/spec/ckb2021",
                    app_config: "testdata/config/ckb2021",
                },
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");

        // Deploy our data cells onto chain.
        let mut deployer = RFC0222CellDeployer::default();
        deployer.deploy(node2021);

        let rfc0222_height = calc_epoch_start_number(node2021, RFC0222_EPOCH_NUMBER);
        node2021.mine_to(rfc0222_height - 10);

        // Build txs
        let tx_valid_2021_only = build_transaction(
            node2021,
            Some(
                // match to always_success_cell_dep_a1 and always_success_cell_dep_b1,
                // always_success_cell_dep_a1 and always_success_cell_dep_b1 have the different data hash
                Script::new_builder()
                    .code_hash(node2021.always_success_script().calc_script_hash())
                    .hash_type(ScriptHashType::Type.into())
                    .build(),
            ),
            vec![
                node2021.always_success_cell_dep(),
                deployer.always_success_cell_dep_a1(),
                deployer.always_success_cell_dep_a2(),
            ],
        );
        let tx_valid_both = build_transaction(
            node2021,
            Some(
                // match to always_success_cell_dep_a1 and always_success_cell_dep_b1,
                // always_success_cell_dep_a1 and always_success_cell_dep_b1 have the different data hash
                Script::new_builder()
                    .code_hash(node2021.always_success_script().calc_script_hash())
                    .hash_type(ScriptHashType::Type.into())
                    .build(),
            ),
            vec![
                node2021.always_success_cell_dep(),
                deployer.always_success_cell_dep_a1(),
            ],
        );

        // Build chain-a
        let chain_a = build_unverified_chain(node2021, rfc0222_height + 1, vec![]);

        // Build chain-b
        let chain_b_tx_valid_2021_only_attach_before_switch = build_unverified_chain(
            node2021,
            rfc0222_height + 2,
            vec![
                BuildUnverifiedChainParam::Proposal {
                    block_number: rfc0222_height - 3,
                    proposal_short_id: tx_valid_2021_only.proposal_short_id(),
                },
                BuildUnverifiedChainParam::Committed {
                    block_number: rfc0222_height - 1,
                    transaction: tx_valid_2021_only.clone(),
                },
            ],
        );
        let chain_b_tx_valid_2021_only_attach_after_switch = build_unverified_chain(
            node2021,
            rfc0222_height + 2,
            vec![
                BuildUnverifiedChainParam::Proposal {
                    block_number: rfc0222_height - 2,
                    proposal_short_id: tx_valid_2021_only.proposal_short_id(),
                },
                BuildUnverifiedChainParam::Committed {
                    block_number: rfc0222_height,
                    transaction: tx_valid_2021_only.clone(),
                },
            ],
        );
        let chain_b_tx_valid_both_attach_before_switch = build_unverified_chain(
            node2021,
            rfc0222_height + 2,
            vec![
                BuildUnverifiedChainParam::Proposal {
                    block_number: rfc0222_height - 3,
                    proposal_short_id: tx_valid_both.proposal_short_id(),
                },
                BuildUnverifiedChainParam::Committed {
                    block_number: rfc0222_height - 1,
                    transaction: tx_valid_both.clone(),
                },
            ],
        );
        let chain_b_tx_valid_both_attach_after_switch = build_unverified_chain(
            node2021,
            rfc0222_height + 2,
            vec![
                BuildUnverifiedChainParam::Proposal {
                    block_number: rfc0222_height - 2,
                    proposal_short_id: tx_valid_both.proposal_short_id(),
                },
                BuildUnverifiedChainParam::Committed {
                    block_number: rfc0222_height,
                    transaction: tx_valid_both.clone(),
                },
            ],
        );

        let chain_bs = vec![
            (
                "chain_b_tx_valid_2021_only_attach_before_switch",
                chain_b_tx_valid_2021_only_attach_before_switch,
                Err(ERROR_MULTIPLE_MATCHES.to_string()),
            ),
            (
                "chain_b_tx_valid_2021_only_attach_after_switch",
                chain_b_tx_valid_2021_only_attach_after_switch,
                Ok(()),
            ),
            (
                "chain_b_tx_valid_both_attach_before_switch",
                chain_b_tx_valid_both_attach_before_switch,
                Ok(()),
            ),
            (
                "chain_b_tx_valid_both_attach_after_switch",
                chain_b_tx_valid_both_attach_after_switch,
                Ok(()),
            ),
        ];

        assert_eq!(nodes.nodes().len(), chain_bs.len());
        // Make base node synced
        for base_node in nodes.nodes() {
            for number in base_node.get_tip_block_number() + 1..=node2021.get_tip_block_number() {
                base_node.submit_block(&node2021.get_block_by_number(number));
            }
        }
        for (base_node, (case_id, chain_b, expected)) in nodes.nodes().zip(chain_bs) {
            RFC0222ReorgAttachedBlocks::check_reorg(
                case_id,
                base_node,
                chain_a.clone(),
                chain_b,
                expected,
            );
        }
    }
}

impl RFC0222ReorgAttachedBlocks {
    pub fn check_reorg(
        case_id: &str,
        node2021: &Node,
        chain_a: Vec<BlockView>,
        chain_b: Vec<BlockView>,
        expected: Result<(), String>,
    ) {
        let chain_a_tip_number = chain_a
            .last()
            .map(|block| block.number())
            .expect("should be ok");
        let chain_a_tip_hash = chain_a
            .last()
            .map(|block| block.hash())
            .expect("should be ok");
        let chain_b_tip_number = chain_b
            .last()
            .map(|block| block.number())
            .expect("should be ok");
        let chain_b_tip_hash = chain_b
            .last()
            .map(|block| block.hash())
            .expect("should be ok");

        // chain_b must be longer than chain_a, it is condition to trigger reorg
        assert!(chain_b_tip_number > chain_a_tip_number);

        for block in chain_a {
            let result = node2021
                .rpc_client()
                .submit_block("".to_string(), block.data().into());
            assert!(
                result.is_ok(),
                "at case \"{}\", sending chain_a's block to {} should be ok, but got {:?}",
                case_id,
                node2021.node_name(),
                result
            );
        }
        assert_eq!(chain_a_tip_number, node2021.get_tip_block().number());
        assert_eq!(chain_a_tip_hash, node2021.get_tip_block().hash());

        let mut expected_reorg_failed_error = None;
        for block in chain_b {
            if expected.is_ok() {
                node2021.submit_block(&block);
            } else {
                let _ = node2021
                    .rpc_client()
                    .submit_block("".to_string(), block.data().into())
                    .map_err(|err| {
                        if expected_reorg_failed_error.is_none() {
                            expected_reorg_failed_error = Some(err);
                        }
                    });
            }
        }

        if expected.is_ok() {
            assert_eq!(chain_b_tip_number, node2021.get_tip_block().number());
            assert_eq!(chain_b_tip_hash, node2021.get_tip_block().hash());
        } else if let Err(errmsg) = expected {
            assert!(
                expected_reorg_failed_error.is_some(),
                "at case \"{}\", expected error contains errmsg \"{}\", but got ok",
                case_id,
                errmsg,
            );
            assert!(
                expected_reorg_failed_error
                    .as_ref()
                    .unwrap()
                    .to_string()
                    .contains(&errmsg),
                "at case \"{}\", expected error contains errmsg\"{}\", but got {:?}",
                case_id,
                errmsg,
                expected_reorg_failed_error.unwrap(),
            );
            assert_ne!(chain_b_tip_number, node2021.get_tip_block().number());
            assert_ne!(chain_b_tip_hash, node2021.get_tip_block().hash());
        }
    }
}
