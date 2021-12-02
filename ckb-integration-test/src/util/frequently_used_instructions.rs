use ckb_testkit::ckb_types::core::{BlockNumber, TransactionView};
use ckb_testkit::BuildInstruction;

/// Return instructions that sends the given transaction when template_number equals
/// `fork_switch_height - 4`
pub fn instructions_to_send_transaction_before_switch(
    fork_switch_height: BlockNumber,
    transaction: TransactionView,
) -> Vec<BuildInstruction> {
    vec![BuildInstruction::SendTransaction {
        template_number: fork_switch_height - 4,
        transaction,
    }]
}

/// Return instructions that commits the given transaction when template_number equals
/// `fork_switch_height - 1`
pub fn instructions_to_commit_transaction_before_switch(
    fork_switch_height: BlockNumber,
    transaction: TransactionView,
) -> Vec<BuildInstruction> {
    vec![
        BuildInstruction::Propose {
            template_number: fork_switch_height - 3,
            proposal_short_id: transaction.proposal_short_id(),
        },
        BuildInstruction::Commit {
            template_number: fork_switch_height - 1,
            transaction,
        },
    ]
}

/// Return instructions that sends the given transaction when template_number equals
/// `fork_switch_height - 2`
pub fn instructions_to_send_transaction_after_switch(
    fork_switch_height: BlockNumber,
    transaction: TransactionView,
) -> Vec<BuildInstruction> {
    vec![BuildInstruction::SendTransaction {
        template_number: fork_switch_height - 2,
        transaction,
    }]
}

/// Return instructions that commits the given transaction when template_number equals
/// `fork_switch_height`
pub fn instructions_to_commit_transaction_after_switch(
    fork_switch_height: BlockNumber,
    transaction: TransactionView,
) -> Vec<BuildInstruction> {
    vec![
        BuildInstruction::Propose {
            template_number: fork_switch_height - 2,
            proposal_short_id: transaction.proposal_short_id(),
        },
        BuildInstruction::Commit {
            template_number: fork_switch_height,
            transaction,
        },
    ]
}
