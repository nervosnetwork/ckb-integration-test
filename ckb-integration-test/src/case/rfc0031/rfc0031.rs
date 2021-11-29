use super::{ERROR_EMPTY_EXT, ERROR_MAX_LIMIT, ERROR_UNKNOWN_FIELDS, RFC0031_EPOCH_NUMBER};
use crate::preclude::*;
use crate::util::estimate_start_number_of_epoch;
use ckb_testkit::assert_result_eq;
use ckb_types::{
    core::{BlockNumber, BlockView},
    packed,
    prelude::*,
};

/// ```text
/// ┌─────────────────────┬───────────────────────┬───────────────────────┐
/// │                     │                       │                       │
/// │ block.extension.len │  v2019                │  v2021                │
/// ├─────────────────────┼───────────────────────┼───────────────────────┤
/// │ None                │  Ok                   │  Ok                   │
/// ├─────────────────────┼───────────────────────┼───────────────────────┤
/// │ Some(0)             │  Err(UNKNOWN_FIELDS)  │  Err(EMPTY_EXT)       │
/// ├─────────────────────┼───────────────────────┼───────────────────────┤
/// │ Some(1)             │  Err(UNKNOWN_FIELDS)  │  Ok                   │
/// ├─────────────────────┼───────────────────────┼───────────────────────┤
/// │ Some(16)            │  Err(UNKNOWN_FIELDS)  │  Ok                   │
/// ├─────────────────────┼───────────────────────┼───────────────────────┤
/// │ Some(32)            │  Err(UNKNOWN_FIELDS)  │  Ok                   │
/// ├─────────────────────┼───────────────────────┼───────────────────────┤
/// │ Some(64)            │  Err(UNKNOWN_FIELDS)  │  Ok                   │
/// ├─────────────────────┼───────────────────────┼───────────────────────┤
/// │ Some(96)            │  Err(UNKNOWN_FIELDS)  │  Ok                   │
/// ├─────────────────────┼───────────────────────┼───────────────────────┤
/// │ Some(97)            │  Err(UNKNOWN_FIELDS)  │  Err(MAX_LIMIT)       │
/// └─────────────────────┴───────────────────────┴───────────────────────┘
/// ```
pub struct RFC0031;

impl Case for RFC0031 {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![NodeOptions {
                node_name: String::from("node2021"),
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/Epoch2V2TestData",
                chain_spec: "testdata/spec/ckb2021",
                app_config: "testdata/config/ckb2021",
            }],
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");

        let fork_switch_height = estimate_start_number_of_epoch(node2021, RFC0031_EPOCH_NUMBER);
        node2021.mine_to(fork_switch_height - 6);

        let cases = vec![
            (0, None, Ok(()), Ok(())),
            (1, Some(0), Err(ERROR_UNKNOWN_FIELDS), Err(ERROR_EMPTY_EXT)),
            (2, Some(1), Err(ERROR_UNKNOWN_FIELDS), Ok(())),
            (3, Some(16), Err(ERROR_UNKNOWN_FIELDS), Ok(())),
            (4, Some(32), Err(ERROR_UNKNOWN_FIELDS), Ok(())),
            (5, Some(64), Err(ERROR_UNKNOWN_FIELDS), Ok(())),
            (6, Some(96), Err(ERROR_UNKNOWN_FIELDS), Ok(())),
            (7, Some(97), Err(ERROR_UNKNOWN_FIELDS), Err(ERROR_MAX_LIMIT)),
        ];
        for (
            case_id,
            extension_size,
            expected_result_before_switch,
            expected_result_after_switch,
        ) in cases
        {
            run_case_before_switch(
                node2021,
                fork_switch_height,
                case_id,
                extension_size,
                expected_result_before_switch,
            );
            run_case_after_switch(
                node2021,
                fork_switch_height,
                case_id,
                extension_size,
                expected_result_after_switch,
            );
        }
    }
}

fn build_block(node: &Node, extension_size: Option<usize>) -> BlockView {
    let template = node.rpc_client().get_block_template(None, None, None);
    packed::Block::from(template)
        .as_advanced_builder()
        .extension(extension_size.map(|s| vec![0u8; s].pack()))
        .build()
}

fn run_case_before_switch(
    node2021: &Node,
    fork_switch_height: BlockNumber,
    case_id: usize,
    extension_size: Option<usize>,
    expected_result_before_switch: Result<(), &str>,
) {
    let node = node2021.clone_node(&format!("case-{}-node2021-before-switch", case_id));

    node.mine_to(fork_switch_height - 2);
    let block = build_block(&node, extension_size);

    let actual_result_before_switch = node
        .rpc_client()
        .submit_block("".to_owned(), block.data().into())
        .map(|_| ());
    assert_result_eq!(
        expected_result_before_switch,
        actual_result_before_switch,
        "case-{} expected_result_before_switch: {:?}, actual_result_before_switch: {:?}",
        case_id,
        expected_result_before_switch,
        actual_result_before_switch,
    );
}

fn run_case_after_switch(
    node2021: &Node,
    fork_switch_height: BlockNumber,
    case_id: usize,
    extension_size: Option<usize>,
    expected_result_after_switch: Result<(), &str>,
) {
    let node = node2021.clone_node(&format!("case-{}-node2021-after-switch", case_id));

    node.mine_to(fork_switch_height - 1);
    let block = build_block(&node, extension_size);

    let actual_result_after_switch = node
        .rpc_client()
        .submit_block("".to_owned(), block.data().into())
        .map(|_| ());
    assert_result_eq!(
        expected_result_after_switch,
        actual_result_after_switch,
        "case-{} expected_result_after_switch: {:?}, actual_result_after_switch: {:?}",
        case_id,
        expected_result_after_switch,
        actual_result_after_switch,
    );
}
