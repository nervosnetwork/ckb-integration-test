// Rfc0222 allows multiple cell dep matches on type script hash when all the matches are resolved
// to the same script code. **This is a looser change**.
//
// ## Detached Transactions
//
// |                | valid for 2021 only | valid for both 2019 and 2021 |
// | :-----         | ----:               | :----:                       |
// | tx is pending  | committed at tip+3  | committed at tip+3           |
// | tx is in gap   | committed at tip+2  | committed at tip+2           |
// | tx is proposed | committed at tip+1  | committed at tip+1           |
//
// - tx_valid_2021_only_pending
// - tx_valid_2021_only_gap
// - tx_valid_2021_only_proposed
// - ~~tx_valid_both_pending~~
// - ~~tx_valid_both_gap~~
// - ~~tx_valid_both_proposed~~
//
// At our test cases, we don't care of where the transactions been committed. After chain
// reorganization, these transactions be detached from canonical chain, re-put into tx-pool. Tx-pool
// will decide to propose/commit/reject depending on circumstances.
//
// ## Case Description
//
// ```
// chain_a: [2997] [2998] [2999] ..(rfc0222 switch).. [3000] [3001]
// chain_b: [2997] [2998] [2999] ..(rfc0222 switch).. [3000] [3001] [3002]
// ```
//
// 1. `node` runs chain_a, height is 3001
//   - after chain_a[2997], before chain_a[2998], send `tx_valid_2021_only_pending`,
//     `tx_valid_2021_only_gap`, `tx_valid_2021_only_proposed` into tx-pool, it should be ok
//     because these transactions is invalid after 2 blocks when them been committed.
//   - expect that chain_a[3000] committed `tx_valid_2021_only_pending`, `tx_valid_2021_only_gap`, `tx_valid_2021_only_proposed`
// 2. Construct a chain_b, height is 3002, chain_b[2998..3002] are empty block transactions.
//   - chain_b[2997..3002].transactions is empty
//   - chain_b[2997..3000].proposals is empty
//   - chain_b[4501] proposed `tx_valid_2021_only_proposed`
//   - chain_b[3002] proposed `tx_valid_2021_only_gap`
// 3. Send chain_b to `node`, `node` will trigger reorg, 3 transactions will be detached and re-put
//    into tx-pool.
// 4. Check transactions statuses:
//   - `tx_valid_2021_only_pending` is pending
//   - `tx_valid_2021_only_gap` is gap
//   - `tx_valid_2021_only_proposed` is proposed
// 5. Mine 1 block, check transactions statuses:
//   - `tx_valid_2021_only_pending` is gap
//   - `tx_valid_2021_only_gap` is proposed
//   - `tx_valid_2021_only_proposed` is committed
// 6. Mine 1 block, check transactions statuses:
//   - `tx_valid_2021_only_pending` is proposed
//   - `tx_valid_2021_only_gap` is committed
//   - `tx_valid_2021_only_proposed` is committed
// 7. Mine 1 block, check transactions statuses:
//   - `tx_valid_2021_only_pending` is committed
//   - `tx_valid_2021_only_gap` is committed
//   - `tx_valid_2021_only_proposed` is committed

use crate::case::rfc0222::util::{
    build_transaction_with_input, RFC0222CellDeployer,
};
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

pub struct RFC0222MultipleTransactions;

impl Case for RFC0222MultipleTransactions {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![NodeOptions {
                node_name: "node2021",
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/Epoch2V2TestData",
                chain_spec: "testdata/spec/ckb2021",
                app_config: "testdata/config/ckb2021",
            }]
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
        node2021.mine_to(rfc0222_height - 3);
        assert_eq!(node2021.get_tip_block_number(), 2997);

        // Build txs
        let inputs = node2021.get_spendable_always_success_cells();
        // NOTE: If only 2 transactions, every is ok; while 3 is error.
        let txs_valid_2021_only = (0..3)
            .map(|index| {
                build_transaction_with_input(
                    &inputs[index],
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
                )
            })
            .collect::<Vec<_>>();

        //   - after chain_a[2997], before chain_a[2998], send `tx_valid_2021_only_pending`,
        //     `tx_valid_2021_only_gap`, `tx_valid_2021_only_proposed` into tx-pool, it should be ok
        //     because these transactions is invalid after 2 blocks when them been committed.
        assert_eq!(node2021.get_tip_block_number(), 2997);
        assert_eq!(
            calc_epoch_start_number(node2021, RFC0222_EPOCH_NUMBER)
                - node2021.consensus().tx_proposal_window.closest.value()
                - 1,
            2997
        );
        for (index, tx) in txs_valid_2021_only.iter().enumerate() {
            let result = node2021
                .rpc_client()
                .send_transaction_result(tx.data().into());
            assert!(
                result.is_ok(),
                "tx-{}, {:#x} is valid after 2 blocks when it been committed so it can be submitted into tx-pool, but got {:?}",
                index,
                tx.hash(),
                result
            );
        }
        node2021.mine(node2021.consensus().tx_proposal_window.closest.value()+1);
        for (index, tx) in txs_valid_2021_only.iter().enumerate() {
            assert!(
                node2021.is_transaction_committed(tx),
                "tx-{} {:#x} should be committed, but got {:?}",
                index, tx.hash(), node2021.rpc_client().get_transaction(tx.hash()),
            );
        }
    }
}
