use super::{ERROR_EMPTY_EXT, ERROR_MAX_LIMIT, ERROR_UNKNOWN_FIELDS, RFC0031_EPOCH_NUMBER};
use crate::prelude::*;
use crate::util::estimate_start_number_of_epoch;
use ckb_testkit::assert_result_eq;
use ckb_testkit::ckb_types::{
    core::{BlockNumber, BlockView},
    packed,
    prelude::*,
};

const RFC0031_BLOCK_NUMBER: BlockNumber = 3000;

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

        for case in self.cases_params() {
            let node = node2021.clone_node(&format!("case-{}-node", case.id));
            node.mine_to(case.height - 1);
            let block = self.build_block(&node, case.extension_size);
            let actual_result = node
                .rpc_client()
                .submit_block("".to_owned(), block.data().into())
                .map(|_| ());
            assert_result_eq!(
                case.expected_result,
                actual_result,
                "case.id: {}, node.log: {}",
                case.id,
                node.log_path().to_string_lossy()
            );
        }
    }
}

struct CaseParams {
    id: usize,
    extension_size: Option<usize>,
    height: BlockNumber,
    expected_result: Result<(), &'static str>,
}

impl RFC0031 {
    fn build_block(&self, node: &Node, extension_size: Option<usize>) -> BlockView {
        let template = node.rpc_client().get_block_template(None, None, None);
        packed::Block::from(template)
            .as_advanced_builder()
            .extension(extension_size.map(|s| vec![0u8; s].pack()))
            .build()
    }

    fn cases_params(&self) -> Vec<CaseParams> {
        vec![
            CaseParams {
                id: 0,
                extension_size: None,
                height: RFC0031_BLOCK_NUMBER - 1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 1,
                extension_size: Some(0),
                height: RFC0031_BLOCK_NUMBER - 1,
                expected_result: Err(ERROR_UNKNOWN_FIELDS),
            },
            CaseParams {
                id: 2,
                extension_size: Some(1),
                height: RFC0031_BLOCK_NUMBER - 1,
                expected_result: Err(ERROR_UNKNOWN_FIELDS),
            },
            CaseParams {
                id: 3,
                extension_size: Some(16),
                height: RFC0031_BLOCK_NUMBER - 1,
                expected_result: Err(ERROR_UNKNOWN_FIELDS),
            },
            CaseParams {
                id: 4,
                extension_size: Some(32),
                height: RFC0031_BLOCK_NUMBER - 1,
                expected_result: Err(ERROR_UNKNOWN_FIELDS),
            },
            CaseParams {
                id: 5,
                extension_size: Some(64),
                height: RFC0031_BLOCK_NUMBER - 1,
                expected_result: Err(ERROR_UNKNOWN_FIELDS),
            },
            CaseParams {
                id: 6,
                extension_size: Some(96),
                height: RFC0031_BLOCK_NUMBER - 1,
                expected_result: Err(ERROR_UNKNOWN_FIELDS),
            },
            CaseParams {
                id: 7,
                extension_size: Some(97),
                height: RFC0031_BLOCK_NUMBER - 1,
                expected_result: Err(ERROR_UNKNOWN_FIELDS),
            },
            CaseParams {
                id: 8,
                extension_size: None,
                height: RFC0031_BLOCK_NUMBER,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 9,
                extension_size: Some(0),
                height: RFC0031_BLOCK_NUMBER,
                expected_result: Err(ERROR_EMPTY_EXT),
            },
            CaseParams {
                id: 10,
                extension_size: Some(1),
                height: RFC0031_BLOCK_NUMBER,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 11,
                extension_size: Some(16),
                height: RFC0031_BLOCK_NUMBER,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 12,
                extension_size: Some(32),
                height: RFC0031_BLOCK_NUMBER,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 13,
                extension_size: Some(64),
                height: RFC0031_BLOCK_NUMBER,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 14,
                extension_size: Some(96),
                height: RFC0031_BLOCK_NUMBER,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 15,
                extension_size: Some(97),
                height: RFC0031_BLOCK_NUMBER,
                expected_result: Err(ERROR_MAX_LIMIT),
            },
        ]
    }
}
