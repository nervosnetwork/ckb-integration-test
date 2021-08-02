use ckb_crypto::secp::Privkey;
use ckb_testkit::{Node, User};
use ckb_types::{
    core::{Capacity, TransactionBuilder},
    packed::{Byte32, CellInput, CellOutput},
    prelude::*,
};
use std::cmp::min;
use std::thread::sleep;
use std::time::Duration;

// TODO handle big cell
pub fn dispatch(nodes: &[Node], lender: &User, borrowers: &[User], borrow_capacity: u64) {
    ckb_testkit::info!(
        "dispatch to {} borrowers, {} capacity per borrower",
        borrowers.len(),
        borrow_capacity
    );
    let live_cells = lender.get_spendable_single_secp256k1_cells(&nodes[0]);
    let mut i_borrower = 0;
    let mut txs = Vec::new();
    for chunk in live_cells.chunks(1) {
        let inputs = chunk;
        let inputs_capacity: u64 = inputs.iter().map(|cell| cell.capacity().as_u64()).sum();
        // TODO estimate tx fee
        let fee = (inputs_capacity / borrow_capacity) * 1000;
        let outputs_capacity = inputs_capacity - fee;
        let n_outputs =
            if (outputs_capacity / borrow_capacity) as usize > borrowers.len() - i_borrower {
                min(1500, borrowers.len() - i_borrower)
            } else {
                min(1500, (outputs_capacity / borrow_capacity) as usize)
            };
        let change_capacity = inputs_capacity - n_outputs as u64 * borrow_capacity - fee;
        let mut outputs = borrowers[i_borrower..i_borrower + n_outputs]
            .iter()
            .map(|borrower| {
                CellOutput::new_builder()
                    .capacity(borrow_capacity.pack())
                    .lock(borrower.single_secp256k1_lock_script())
                    .build()
            })
            .collect::<Vec<_>>();
        if change_capacity >= Capacity::bytes(67).unwrap().as_u64() {
            let change_output = CellOutput::new_builder()
                .capacity(change_capacity.pack())
                .lock(lender.single_secp256k1_lock_script())
                .build();
            outputs.push(change_output);
        }
        let outputs_data = (0..outputs.len())
            .map(|_| Default::default())
            .collect::<Vec<_>>();
        let unsigned_tx = TransactionBuilder::default()
            .inputs(
                inputs
                    .iter()
                    .map(|cell| CellInput::new(cell.out_point.clone(), 0)),
            )
            .outputs(outputs)
            .outputs_data(outputs_data)
            .cell_dep(lender.single_secp256k1_cell_dep())
            .build();
        let witness = lender
            .single_secp256k1_signed_witness(&unsigned_tx)
            .as_bytes()
            .pack();
        let signed_tx = unsigned_tx
            .as_advanced_builder()
            .set_witnesses(vec![witness])
            .build();

        txs.push(signed_tx);
        i_borrower += n_outputs;
        if i_borrower >= borrowers.len() {
            break;
        }
    }

    let total_capacity: u64 = live_cells.iter().map(|cell| cell.capacity().as_u64()).sum();
    assert!(
        i_borrower >= borrowers.len(),
        "lender has not enough capacity for borrowers, total_capacity: {}, rest {} borrowers",
        total_capacity,
        borrowers.len().saturating_sub(i_borrower),
    );
    for tx in txs {
        while let Err(err) = nodes[0]
            .rpc_client()
            .send_transaction_result(tx.data().into())
        {
            ckb_testkit::debug!(
                "failed to send transaction {:#x}, error: {}",
                tx.hash(),
                err
            );
            sleep(Duration::from_secs(1));
        }
    }
}

pub fn collect(nodes: &[Node], lender: &User, borrowers: &[User]) {
    ckb_testkit::info!("collect {} borrowers' capacity", borrowers.len());
    let mut txs = Vec::new();
    for borrower in borrowers.iter() {
        let live_cells = borrower.get_spendable_single_secp256k1_cells(&nodes[0]);
        if live_cells.is_empty() {
            continue;
        }
        for chunk in live_cells.chunks(100) {
            let inputs = chunk;
            let inputs_capacity: u64 = inputs.iter().map(|cell| cell.capacity().as_u64()).sum();
            // TODO estimate tx fee
            let fee = inputs.len() as u64 * 1000;
            let output = CellOutput::new_builder()
                .capacity((inputs_capacity - fee).pack())
                .lock(lender.single_secp256k1_lock_script())
                .build();
            let unsigned_tx = TransactionBuilder::default()
                .inputs(
                    inputs
                        .iter()
                        .map(|cell| CellInput::new(cell.out_point.clone(), 0)),
                )
                .output(output)
                .output_data(Default::default())
                .cell_dep(lender.single_secp256k1_cell_dep())
                .build();
            let witness = borrower
                .single_secp256k1_signed_witness(&unsigned_tx)
                .as_bytes()
                .pack();
            let signed_tx = unsigned_tx
                .as_advanced_builder()
                .set_witnesses(vec![witness])
                .build();
            txs.push(signed_tx);
        }
    }

    for tx in txs {
        while let Err(err) = nodes[0]
            .rpc_client()
            .send_transaction_result(tx.data().into())
        {
            ckb_testkit::debug!(
                "failed to send transaction {:#x}, error: {}",
                tx.hash(),
                err
            );
            sleep(Duration::from_secs(1));
        }
    }
}

pub fn generate_privkeys(basic_raw_privkey: Byte32, n: usize) -> Vec<Privkey> {
    let mut raw_privkeys = (0..n).fold(vec![basic_raw_privkey], |mut raw_privkeys, _| {
        let last_raw_privkey = raw_privkeys.last().unwrap();
        let next_raw_privkey = ckb_hash::blake2b_256(last_raw_privkey.as_bytes()).pack();
        raw_privkeys.push(next_raw_privkey);
        raw_privkeys
    });
    raw_privkeys = raw_privkeys.split_off(1);
    raw_privkeys
        .iter()
        .map(|raw_privkey| Privkey::from_slice(raw_privkey.as_slice()))
        .collect()
}
