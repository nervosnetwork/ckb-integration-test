use crate::preclude::*;
use crate::util::frequently_used_instructions::{
    instructions_to_commit_transaction_after_switch,
    instructions_to_commit_transaction_before_switch,
    instructions_to_send_transaction_after_switch, instructions_to_send_transaction_before_switch,
};
use ckb_testkit::assert_result_eq;
use ckb_types::core::{BlockNumber, TransactionView};

pub fn run_case_before_switch(
    node2021: &Node,
    fork_switch_height: BlockNumber,
    case_id: usize,
    tx: &TransactionView,
    expected_result_before_switch: Result<(), &str>,
) {
    assert!(node2021.rpc_client().ckb2021);
    assert!(node2021.get_tip_block_number() <= fork_switch_height - 4);

    {
        let node = node2021.clone_node(&format!("case-{}-node2021-before-switch", case_id));
        let ins = instructions_to_send_transaction_before_switch(fork_switch_height, tx);
        let actual_result_before_switch =
            node.build_according_to_instructions(fork_switch_height + 10, ins);
        assert_result_eq!(
            expected_result_before_switch,
            actual_result_before_switch,
            "case-{} expected_result_before_switch = {:?}, actual_result_before_switch: {:?}",
            case_id,
            expected_result_before_switch,
            actual_result_before_switch,
        );
    }

    {
        let node = node2021.clone_node(&format!("case-{}-node2021-before-switch", case_id));
        let ins = instructions_to_commit_transaction_before_switch(fork_switch_height, tx);
        let actual_result_before_switch =
            node.build_according_to_instructions(fork_switch_height + 10, ins);
        assert_result_eq!(
            expected_result_before_switch,
            actual_result_before_switch,
            "case-{} expected_result_before_switch = {:?}, actual_result_before_switch: {:?}",
            case_id,
            expected_result_before_switch,
            actual_result_before_switch,
        );
    }
}

pub fn run_case_after_switch(
    node2021: &Node,
    fork_switch_height: BlockNumber,
    case_id: usize,
    tx: &TransactionView,
    expected_result_after_switch: Result<(), &str>,
) {
    assert!(node2021.rpc_client().ckb2021);
    assert!(node2021.get_tip_block_number() <= fork_switch_height - 4);

    {
        let node = node2021.clone_node(&format!("case-{}-node2021-after-switch", case_id));
        let ins = instructions_to_send_transaction_after_switch(fork_switch_height, tx);
        let actual_result_after_switch =
            node.build_according_to_instructions(fork_switch_height + 10, ins);
        assert_result_eq!(
            expected_result_after_switch,
            actual_result_after_switch,
            "case-{} expected_result_after_switch = {:?}, actual_result_after_switch: {:?}",
            case_id,
            expected_result_after_switch,
            actual_result_after_switch,
        );
    }

    {
        let node = node2021.clone_node(&format!("case-{}-node2021-after-switch", case_id));
        let ins = instructions_to_commit_transaction_after_switch(fork_switch_height, tx);
        let actual_result_after_switch =
            node.build_according_to_instructions(fork_switch_height + 10, ins);
        assert_result_eq!(
            expected_result_after_switch,
            actual_result_after_switch,
            "case-{} expected_result_after_switch = {:?}, actual_result_after_switch: {:?}",
            case_id,
            expected_result_after_switch,
            actual_result_after_switch,
        );
    }
}
