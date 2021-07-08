// Dispatch miner's capacity to users.
//
// Every user receives 10000CKB capacity

use ckb_crypto::secp::Privkey;
use ckb_testkit::{Node, User};
use ckb_types::{
    core::{Capacity, TransactionBuilder},
    packed::{Byte32, CellInput, CellOutput},
    prelude::*,
};

pub fn dispatch(nodes: &[Node], lender: &User, borrowers: &[User], borrow_capacity: u64) {
    // TODO return input cells intersection of live cells of nodes
    let live_cells = lender.get_live_single_secp256k1_cells(&nodes[0]);
    let mut i = 0;
    let mut txs = Vec::new();
    for chunk in live_cells.chunks(100) {
        if i >= borrowers.len() {
            break;
        }

        let inputs = chunk;
        let inputs_capacity: u64 = inputs.iter().map(|cell| cell.capacity().as_u64()).sum();
        // TODO estimate tx fee
        let fee = (inputs_capacity / borrow_capacity) * 1000;
        let n_borrowers = ((inputs_capacity - fee) / borrow_capacity) as usize;
        let change_capacity = inputs_capacity - n_borrowers as u64 * borrow_capacity - fee;
        let mut outputs = borrowers[i..=i + n_borrowers - 1]
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
        let tx = TransactionBuilder::default()
            .inputs(
                inputs
                    .iter()
                    .map(|cell| CellInput::new(cell.out_point.clone(), 0)),
            )
            .outputs(outputs)
            .outputs_data(outputs_data)
            .cell_dep(lender.single_secp256k1_cell_dep())
            .build();
        txs.push(tx);

        i = i + n_borrowers;
    }

    assert!(
        i < borrowers.len(),
        "lender has not enough capacity for borrowers"
    );
    for tx in txs {
        nodes[0].submit_transaction(&tx);
    }
}

pub fn collect(nodes: &[Node], lender: &User, borrowers: &[User]) {
    let mut txs = Vec::new();
    for borrower in borrowers.iter() {
        // TODO return input cells intersection of live cells of nodes
        let live_cells = borrower.get_live_single_secp256k1_cells(&nodes[0]);
        for chunk in live_cells.chunks(100) {
            let inputs = chunk;
            let inputs_capacity: u64 = inputs.iter().map(|cell| cell.capacity().as_u64()).sum();
            // TODO estimate tx fee
            let fee = inputs.len() as u64 * 1000;
            let output = CellOutput::new_builder()
                .capacity((inputs_capacity - fee).pack())
                .lock(lender.single_secp256k1_lock_script())
                .build();
            let tx = TransactionBuilder::default()
                .inputs(
                    inputs
                        .iter()
                        .map(|cell| CellInput::new(cell.out_point.clone(), 0)),
                )
                .output(output)
                .output_data(Default::default())
                .cell_dep(lender.single_secp256k1_cell_dep())
                .build();
            txs.push(tx);
        }
    }

    for tx in txs {
        nodes[0].submit_transaction(&tx);
    }
}

pub fn generate_privkeys(basic_raw_privkey: Byte32, n: usize) -> Vec<Privkey> {
    let raw_privkeys = (0..n).fold(vec![basic_raw_privkey], |mut raw_privkeys, _| {
        let last_raw_privkey = raw_privkeys.last().unwrap();
        let next_raw_privkey = ckb_hash::blake2b_256(last_raw_privkey.as_bytes()).pack();
        raw_privkeys.push(next_raw_privkey);
        raw_privkeys
    });
    raw_privkeys
        .iter()
        .map(|raw_privkey| Privkey::from_slice(raw_privkey.as_slice()))
        .collect()
}
