use super::{ERROR_IMMATURE, ERROR_INVALID_SINCE};
use crate::preclude::*;
use crate::util::estimate_start_number_of_epoch;
use ckb_testkit::ckb_types::{
    core::{BlockNumber, Capacity, EpochNumberWithFraction, TransactionBuilder, TransactionView},
    packed::{CellInput, CellOutput, OutPoint},
    prelude::*,
};
use ckb_testkit::util::{
    since_from_absolute_epoch_number_with_fraction, since_from_relative_epoch_number_with_fraction,
};
use ckb_testkit::{assert_result_eq, BuildInstruction};

#[derive(Debug)]
struct CaseParams {
    id: usize,
    since: u64,
    expected_result: Result<EpochNumberWithFraction, &'static str>,
}

/// ## Note
///
/// Our test target is that makes sure:
///
/// * Fork feature is activated at expected height
/// * Some kinds of transactions are ok both before fork and after fork
/// * Some kinds of transactions are immature before fork, then ok after fork
/// * Some kinds of transactions are immature before fork, then invalid after fork
///
/// ## Convention
///
/// * `abs(x, y, z)` is shortcut of `since` that
///   - `since.metric_flag` is epoch (01)
///   - `since.relative_flag` is absolute (0)
///   - `since.value` is `EpochNumberWithFraction { number = x, index = y, length = z }`
///
/// * `rel(x, y, z)` is shortcut of `since` that
///   - `since.metric_flag` is epoch (01)
///   - `since.relative_flag` is relative (1)
///   - `since.value` is `EpochNumberWithFraction { number = x, index = y, length = z }`
///
/// * All epochs length are `1000`
///
/// * The fork feature is activated at `EpochNumberWithFraction(3, 0, 1000)`
///
/// * The transaction input satisfy `input.tx_info.block.epoch == EpochNumberWithFraction(1, 0, 1000)`
///
/// ## Normal Cases
///
/// ```text
/// ┌────┬──────────────────────────────────────┬───────────────────┬─────────────────┐
/// │ id │ since.epoch_f (number, index, length)│  v2019            │  v2021          │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 0  │ abs(2, 0, 0)                         │  Ok(2, 0, 1000)   │  <-             │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 1  │ abs(2, 1, 0)                         │  Ok(2, 0, 1000)   │  InvalidSince   │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 2  │ abs(2, 0, 1)                         │  Ok(2, 0, 1000)   │  <-             │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 3  │ abs(1, 1, 1)                         │  Ok(2, 0, 1000)   │  InvalidSince   │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 4  │ abs(1, 2, 1)                         │  Ok(2, 0, 1000)   │  InvalidSince   │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 5  │ abs(2, 1, 2)                         │  Ok(2, 500, 1000) │  <-             │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 6  │ rel(1, 0, 0)                         │  Ok(2, 0, 1000)   │  <-             │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 7  │ rel(1, 1, 0)                         │  Ok(2, 0, 1000)   │  InvalidSince   │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 8  │ rel(1, 0, 1)                         │  Ok(2, 0, 1000)   │  <-             │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 9  │ rel(0, 1, 1)                         │  Ok(2, 0, 1000)   │  InvalidSince   │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 10 │ rel(0, 5, 4)                         │  Ok(2, 250, 1000) │  InvalidSince   │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 11 │ rel(0, 1, 2)                         │  Ok(1, 500, 1000) │  <-             │
/// └────┴──────────────────────────────────────┴───────────────────┴─────────────────┘
/// ```
///
/// ## Edge Cases
///
/// ```text
/// ┌────┬──────────────────────────────────────┬───────────────────┬─────────────────┐
/// │ id │ since.epoch_f (number, index, length)│  v2019            │  v2021          │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 12 │ abs(2,  999, 1000)                   │  Ok(2, 999, 1000) │  <-             │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 13 │ abs(3,    0, 1000)                   │  Err(Immature)    │  Ok(3, 0, 1000) │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 14 │ abs(3,    0,    0)                   │  Err(Immature)    │  Ok(3, 0, 1000) │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 15 │ abs(3,    0,    1)                   │  Err(Immature)    │  Ok(3, 0, 1000) │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 16 │ abs(3,    1,    0)                   │  Err(Immature)    │  Err(Invalid)   │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 17 │ abs(3,    1,    1)                   │  Err(Immature)    │  Err(Invalid)   │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 18 │ abs(3, 1001, 1000)                   │  Err(Immature)    │  Err(Invalid)   │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 19 │ rel(1,  999, 1000)                   │  Ok(2, 999, 1000) │  <-             │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 20 │ rel(2,    0, 1000)                   │  Err(Immature)    │  Ok(3, 0, 1000) │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 21 │ rel(2,    0,    0)                   │  Err(Immature)    │  Ok(3, 0, 1000) │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 22 │ rel(2,    0,    1)                   │  Err(Immature)    │  Ok(3, 0, 1000) │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 23 │ rel(2,    1,    0)                   │  Err(Immature)    │  Err(Invalid)   │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 24 │ rel(2,    1,    1)                   │  Err(Immature)    │  Err(Invalid)   │
/// ├────┼──────────────────────────────────────┼───────────────────┼─────────────────┤
/// │ 25 │ rel(2, 1001, 1000)                   │  Err(Immature)    │  Err(Invalid)   │
/// └────┴──────────────────────────────────────┴───────────────────┴─────────────────┘
/// ```
pub struct RFC0030;

impl Case for RFC0030 {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![NodeOptions {
                node_name: String::from("node2021"),
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/empty",
                chain_spec: "testdata/spec/ckb2021",
                app_config: "testdata/config/ckb2021",
            }],
        }
    }

    // NOTE: This test make a strong assumption that
    // `input.tx_info.block.epoch == EpochNumberWithFraction(1, 0, 1000)`
    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");

        // Construct input out point which
        // `input.tx_info.block.epoch == EpochNumberWithFraction(1, 0, 1000)
        assert!(node2021.get_tip_block().epoch() <= EpochNumberWithFraction::new(1, 0, 1000));
        let input_out_point = {
            let height = estimate_start_number_of_epoch(node2021, 1);
            assert!(node2021.get_tip_block_number() <= height);
            node2021.mine_to(height);

            let tip_block = node2021.get_tip_block();
            assert_eq!(tip_block.epoch(), EpochNumberWithFraction::new(1, 0, 1000));
            assert_eq!(EpochNumberWithFraction::new(1, 0, 1000), tip_block.epoch(),);

            let tip_cellbase_hash = tip_block
                .transaction(0)
                .expect("cellbase transaction")
                .hash();
            OutPoint::new(tip_cellbase_hash, 0)
        };

        for case in self.cases_params() {
            let tx = build_transaction(node2021, &input_out_point, case.since);
            if let Ok(expected_mature_epoch) = case.expected_result {
                let node = node2021.clone_node(&format!("case-{}-node2021", case.id));
                let mature_height = epoch_to_height(&node, expected_mature_epoch);
                let immature_height = mature_height - 1;
                let immature_result = node.build_according_to_instructions(
                    immature_height,
                    vec![
                        BuildInstruction::Propose {
                            proposal_short_id: tx.proposal_short_id(),
                            template_number: immature_height - 2,
                        },
                        BuildInstruction::Commit {
                            transaction: tx.clone(),
                            template_number: immature_height,
                        },
                    ],
                );
                assert_result_eq!(
                    Result::<(), &str>::Err(ERROR_IMMATURE),
                    immature_result,
                    "case.id: {}, node.log: {}",
                    case.id,
                    node.log_path().to_string_lossy()
                );

                let mature_result = node.build_according_to_instructions(
                    mature_height,
                    vec![BuildInstruction::Commit {
                        transaction: tx.clone(),
                        template_number: mature_height,
                    }],
                );
                assert_eq!(
                    Result::<(), String>::Ok(()),
                    mature_result,
                    "case.id: {}, node.log: {}",
                    case.id,
                    node.log_path().to_string_lossy()
                );
            }
        }
    }
}

impl RFC0030 {
    fn cases_params(&self) -> Vec<CaseParams> {
        vec![
            CaseParams {
                id: 0,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 0, 0),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 1,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 1, 0),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 2,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 0, 1),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 3,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 1, 1),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 4,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 2, 1),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 5,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 1, 2),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(
                    2,
                    1000 * 1 / 2,
                    1000,
                )),
            },
            CaseParams {
                id: 6,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 0, 0),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 7,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 1, 0),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 8,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 0, 1),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 9,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(0, 1, 1),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 10,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(0, 5, 4),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 250, 1000)),
            },
            CaseParams {
                id: 11,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(0, 1, 2),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(1, 500, 1000)),
            },
            CaseParams {
                id: 12,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 999, 1000),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 999, 1000)),
            },
            CaseParams {
                id: 13,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 0, 1000),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 14,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 0, 0),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 15,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 0, 1),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 16,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 1, 0),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 17,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 1, 1),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 18,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 1001, 1000),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 19,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 999, 1000),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 999, 1000)),
            },
            CaseParams {
                id: 20,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 0, 1000),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 21,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 0, 0),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 22,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 0, 1),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 23,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 1, 0),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 24,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 1, 1),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 25,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 1001, 1000),
                ),
                expected_result: Err(ERROR_IMMATURE),
            },
            CaseParams {
                id: 0,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 0, 0),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 1,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 1, 0),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
            CaseParams {
                id: 2,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 0, 1),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 3,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 1, 1),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
            CaseParams {
                id: 4,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 2, 1),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
            CaseParams {
                id: 5,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 1, 2),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(
                    2,
                    1000 * 1 / 2,
                    1000,
                )),
            },
            CaseParams {
                id: 6,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 0, 0),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 7,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 1, 0),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
            CaseParams {
                id: 8,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 0, 1),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 0, 1000)),
            },
            CaseParams {
                id: 9,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(0, 1, 1),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
            CaseParams {
                id: 10,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(0, 5, 4),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
            CaseParams {
                id: 11,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(0, 1, 2),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(1, 500, 1000)),
            },
            CaseParams {
                id: 12,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 999, 1000),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 999, 1000)),
            },
            CaseParams {
                id: 13,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 0, 1000),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(3, 0, 1000)),
            },
            CaseParams {
                id: 14,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 0, 0),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(3, 0, 1000)),
            },
            CaseParams {
                id: 15,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 0, 1),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(3, 0, 1000)),
            },
            CaseParams {
                id: 16,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 1, 0),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
            CaseParams {
                id: 17,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 1, 1),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
            CaseParams {
                id: 18,
                since: since_from_absolute_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(3, 1001, 1000),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
            CaseParams {
                id: 19,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(1, 999, 1000),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(2, 999, 1000)),
            },
            CaseParams {
                id: 20,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 0, 1000),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(3, 0, 1000)),
            },
            CaseParams {
                id: 21,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 0, 0),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(3, 0, 1000)),
            },
            CaseParams {
                id: 22,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 0, 1),
                ),
                expected_result: Ok(EpochNumberWithFraction::new_unchecked(3, 0, 1000)),
            },
            CaseParams {
                id: 23,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 1, 0),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
            CaseParams {
                id: 24,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 1, 1),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
            CaseParams {
                id: 25,
                since: since_from_relative_epoch_number_with_fraction(
                    EpochNumberWithFraction::new_unchecked(2, 1001, 1000),
                ),
                expected_result: Err(ERROR_INVALID_SINCE),
            },
        ]
    }
}

fn build_transaction(node: &Node, input_out_point: &OutPoint, since: u64) -> TransactionView {
    TransactionBuilder::default()
        .input(CellInput::new(input_out_point.clone(), since))
        .output(
            CellOutput::new_builder()
                .lock(node.always_success_script())
                .build_exact_capacity(Capacity::zero())
                .unwrap(),
        )
        .output_data(Default::default())
        .cell_dep(node.always_success_cell_dep())
        .build()
}

fn epoch_to_height(node: &Node, epoch: EpochNumberWithFraction) -> BlockNumber {
    assert!(node.consensus().permanent_difficulty_in_dummy);
    assert_eq!(epoch.length(), 1000);
    epoch.number() * epoch.length() + epoch.index()
}
