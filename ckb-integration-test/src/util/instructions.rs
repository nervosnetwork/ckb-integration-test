use ckb_testkit::BuildInstruction;
use ckb_types::core::{BlockNumber, TransactionView};

pub fn instructions_of_success_to_send_transaction_before_switch(
    fork_switch_height: BlockNumber,
    transaction: &TransactionView,
) -> Vec<BuildInstruction> {
    vec![
        BuildInstruction::SendTransaction {
            block_number: fork_switch_height - 4,
            transaction: transaction.clone(),
        },
        BuildInstruction::Propose {
            block_number: fork_switch_height - 3,
            proposal_short_id: transaction.proposal_short_id(),
        },
        BuildInstruction::Commit {
            block_number: fork_switch_height - 1,
            transaction: transaction.clone(),
        },
    ]
}

pub fn instructions_of_failed_to_send_transaction_before_switch(
    fork_switch_height: BlockNumber,
    transaction: &TransactionView,
) -> Vec<BuildInstruction> {
    vec![BuildInstruction::SendTransaction {
        block_number: fork_switch_height - 4,
        transaction: transaction.clone(),
    }]
}

pub fn instructions_of_failed_to_commit_transaction_before_switch(
    fork_switch_height: BlockNumber,
    transaction: &TransactionView,
) -> Vec<BuildInstruction> {
    vec![
        BuildInstruction::Propose {
            block_number: fork_switch_height - 3,
            proposal_short_id: transaction.proposal_short_id(),
        },
        BuildInstruction::Commit {
            block_number: fork_switch_height - 1,
            transaction: transaction.clone(),
        },
    ]
}

pub fn instructions_of_success_to_send_transaction_after_switch(
    fork_switch_height: BlockNumber,
    transaction: &TransactionView,
) -> Vec<BuildInstruction> {
    vec![
        BuildInstruction::SendTransaction {
            // FIXME I am debugging
            block_number: fork_switch_height - 3,
            // block_number: fork_switch_height - 2,
            transaction: transaction.clone(),
        },
        BuildInstruction::Propose {
            block_number: fork_switch_height - 2,
            proposal_short_id: transaction.proposal_short_id(),
        },
        BuildInstruction::Commit {
            block_number: fork_switch_height,
            transaction: transaction.clone(),
        },
    ]
}

pub fn instructions_of_failed_to_send_transaction_after_switch(
    fork_switch_height: BlockNumber,
    transaction: &TransactionView,
) -> Vec<BuildInstruction> {
    vec![BuildInstruction::SendTransaction {
        block_number: fork_switch_height - 3,
        transaction: transaction.clone(),
    }]
}

pub fn instructions_of_failed_to_commit_transaction_after_switch(
    fork_switch_height: BlockNumber,
    transaction: &TransactionView,
) -> Vec<BuildInstruction> {
    vec![
        BuildInstruction::Propose {
            block_number: fork_switch_height - 2,
            proposal_short_id: transaction.proposal_short_id(),
        },
        BuildInstruction::Commit {
            block_number: fork_switch_height,
            transaction: transaction.clone(),
        },
    ]
}
