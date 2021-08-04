use crate::utils::maybe_retry_send_transaction;
use ckb_crypto::secp::Privkey;
use ckb_testkit::{Node, User};
use ckb_types::core::cell::CellMeta;
use ckb_types::packed::OutPoint;
use ckb_types::{
    core::{Capacity, TransactionBuilder},
    packed::{Byte32, CellInput, CellOutput},
    prelude::*,
};
use std::cmp::min;
use std::collections::VecDeque;

/// count of two-in-two-out txs a block should capable to package.
const TWO_IN_TWO_OUT_COUNT: u64 = 1_000;
const MAX_OUT_COUNT: u64 = TWO_IN_TWO_OUT_COUNT;
const FEE_RATE_OF_OUTPUT: u64 = 1000;

// TODO handle big cell
pub fn dispatch(
    nodes: &[Node],
    owner: &User,
    users: &[User],
    cells_per_user: u64,
    capacity_per_cell: u64,
) {
    ckb_testkit::info!(
        "dispatch to {} users, {} cells per user, {} capacity per cell",
        users.len(),
        cells_per_user,
        capacity_per_cell
    );

    let mut live_cells: VecDeque<CellMeta> = owner
        .get_spendable_single_secp256k1_cells(&nodes[0])
        .into_iter()
        .collect();

    {
        let total_capacity: u64 = live_cells.iter().map(|cell| cell.capacity().as_u64()).sum();
        let total_fee = users.len() as u64 * cells_per_user * FEE_RATE_OF_OUTPUT;
        let need_capacity = users.len() as u64 * cells_per_user * capacity_per_cell + total_fee;
        assert!(
            total_capacity > need_capacity,
            "insufficient capacity, owner's total_capacity({}) <= {} = n_users({}) * cells_per_user({}) * capacity_per_cell({}) + total_fee({})",
            total_capacity,
            need_capacity,
            users.len(),
            cells_per_user,
            capacity_per_cell,
            total_fee,
        );
    }

    let total_outs = users.len() * cells_per_user as usize;
    let index_user = |out_i: usize| out_i % (cells_per_user as usize);

    let mut i_out = 0usize;
    let mut txs = Vec::new();
    while let Some(input) = live_cells.pop_front() {
        let input_capacity = input.capacity().as_u64();
        // TODO estimate tx fee
        let fee = MAX_OUT_COUNT * FEE_RATE_OF_OUTPUT;
        let outputs_capacity = input_capacity - fee;
        let mut n_outs = min(MAX_OUT_COUNT, outputs_capacity / capacity_per_cell) as usize;
        if i_out + n_outs >= total_outs {
            n_outs = total_outs - i_out;
        }
        let change_capacity = outputs_capacity - n_outs as u64 * capacity_per_cell;

        let mut outputs = Vec::with_capacity(n_outs as usize + 1);
        if change_capacity >= Capacity::bytes(67).unwrap().as_u64() {
            let change_output = CellOutput::new_builder()
                .capacity(change_capacity.pack())
                .lock(owner.single_secp256k1_lock_script())
                .build();
            outputs.push(change_output);
        }
        for i in i_out..i_out + n_outs {
            let user = &users[index_user(i)];
            let cell_output = CellOutput::new_builder()
                .capacity(capacity_per_cell.pack())
                .lock(user.single_secp256k1_lock_script())
                .build();
            outputs.push(cell_output);
        }

        let signed_tx = {
            let unsigned_tx = TransactionBuilder::default()
                .input(CellInput::new(input.out_point.clone(), 0))
                .outputs_data(
                    (0..outputs.len())
                        .map(|_| Default::default())
                        .collect::<Vec<_>>(),
                )
                .outputs(outputs)
                .cell_dep(owner.single_secp256k1_cell_dep())
                .build();
            let witness = owner
                .single_secp256k1_signed_witness(&unsigned_tx)
                .as_bytes()
                .pack();
            unsigned_tx
                .as_advanced_builder()
                .set_witnesses(vec![witness])
                .build()
        };

        txs.push(signed_tx.clone());
        i_out += n_outs;
        if i_out == total_outs {
            break;
        }
        if signed_tx.outputs().len() > n_outs {
            // the 1st output is a change cell, push it back into live_cells as it is a live cell
            let change_live_cell = {
                let cell_output = signed_tx.output(0).expect("1st output exists");
                let out_point = OutPoint::new(signed_tx.hash(), 0);
                CellMeta {
                    cell_output,
                    out_point,
                    ..Default::default()
                }
            };
            live_cells.push_back(change_live_cell);
        }
    }

    assert!(i_out == total_outs);
    for tx in txs {
        let result = maybe_retry_send_transaction(&nodes[0], &tx);
        assert!(
            result.is_ok(),
            "dispatch-transaction should be ok but got {}",
            result.unwrap_err()
        );
    }
}

pub fn collect(nodes: &[Node], owner: &User, users: &[User]) {
    ckb_testkit::info!("collect {} users' capacity", users.len());
    let mut txs = Vec::new();
    for user in users.iter() {
        let live_cells = user.get_spendable_single_secp256k1_cells(&nodes[0]);
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
                .lock(owner.single_secp256k1_lock_script())
                .build();
            let unsigned_tx = TransactionBuilder::default()
                .inputs(
                    inputs
                        .iter()
                        .map(|cell| CellInput::new(cell.out_point.clone(), 0)),
                )
                .output(output)
                .output_data(Default::default())
                .cell_dep(owner.single_secp256k1_cell_dep())
                .build();
            let witness = user
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
        let result = maybe_retry_send_transaction(&nodes[0], &tx);
        assert!(
            result.is_ok(),
            "collect-transaction should be ok but got {}",
            result.unwrap_err()
        );
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
