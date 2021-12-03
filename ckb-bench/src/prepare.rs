use crate::utils::maybe_retry_send_transaction;
use ckb_testkit::ckb_crypto::secp::Privkey;
use ckb_testkit::ckb_jsonrpc_types::Status;
use ckb_testkit::ckb_types::core::cell::CellMeta;
use ckb_testkit::ckb_types::packed::OutPoint;
use ckb_testkit::ckb_types::{
    core::{Capacity, TransactionBuilder},
    packed::{Byte32, CellInput, CellOutput},
    prelude::*,
};
use ckb_testkit::{Node, User};
use std::cmp::min;
use std::collections::VecDeque;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// count of two-in-two-out txs a block should capable to package.
pub const TWO_IN_TWO_OUT_COUNT: u64 = 1_000;
pub const MAX_OUT_COUNT: u64 = TWO_IN_TWO_OUT_COUNT;
pub const FEE_RATE_OF_OUTPUT: u64 = 1000;

pub fn dispatch(
    nodes: &[Node],
    owner: &User,
    users: &[User],
    cells_per_user: u64,
    capacity_per_cell: u64,
) {
    ckb_testkit::info!(
        "dispatch with params --n-users {} --cells-per-user {} --capacity-per-cell {}",
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
            "insufficient capacity, owner's total_capacity({}) <= need_capacity({}) = n_users({}) * cells_per_user({}) * capacity_per_cell({}) + total_fee({})",
            total_capacity,
            need_capacity,
            users.len(),
            cells_per_user,
            capacity_per_cell,
            total_fee,
        );
    }

    let total_outs = users.len() * cells_per_user as usize;
    let index_user = |out_i: usize| out_i / (cells_per_user as usize);

    let mut last_logging_time = Instant::now();
    let mut i_out = 0usize;
    let mut inputs = Vec::new();
    let mut txs = Vec::new();
    while let Some(input) = live_cells.pop_front() {
        inputs.push(input);

        let inputs_capacity: u64 = inputs.iter().map(|input| input.capacity().as_u64()).sum();
        // TODO estimate tx fee
        let fee = MAX_OUT_COUNT * FEE_RATE_OF_OUTPUT;
        let outputs_capacity = inputs_capacity - fee;
        let mut n_outs = min(MAX_OUT_COUNT, outputs_capacity / capacity_per_cell) as usize;
        if n_outs == 0 {
            continue;
        }

        if i_out + n_outs >= total_outs {
            n_outs = total_outs - i_out;
        }
        let change_capacity = outputs_capacity - n_outs as u64 * capacity_per_cell;

        let mut outputs = Vec::with_capacity(n_outs as usize + 1);
        if change_capacity >= Capacity::bytes(67).unwrap().as_u64() {
            let change_output = CellOutput::new_builder()
                .capacity(change_capacity.pack())
                .lock(owner.single_secp256k1_lock_script_via_data())
                .build();
            outputs.push(change_output);
        }
        for i in i_out..i_out + n_outs {
            let user = &users[index_user(i)];
            let cell_output = CellOutput::new_builder()
                .capacity(capacity_per_cell.pack())
                .lock(user.single_secp256k1_lock_script_via_data())
                .build();
            outputs.push(cell_output);
        }

        let signed_tx = {
            let unsigned_tx = TransactionBuilder::default()
                .inputs(
                    inputs
                        .iter()
                        .map(|input| CellInput::new(input.out_point.clone(), 0)),
                )
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

        let result = maybe_retry_send_transaction(&nodes[0], &signed_tx);
        if last_logging_time.elapsed() > Duration::from_secs(30) {
            last_logging_time = Instant::now();
            ckb_testkit::info!("dispatching {}/{} outputs", i_out + 1, total_outs)
        }
        assert!(
            result.is_ok(),
            "sending dispatch-transaction {:#x} should be ok but got {}",
            signed_tx.hash(),
            result.unwrap_err()
        );

        // Reset `inputs`, it already been using.
        inputs = Vec::new();
        txs.push(signed_tx.clone());
        i_out += n_outs;
        if i_out == total_outs {
            break;
        }

        // Reuse the change output, we can construct chained transactions
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

    let sent_n_transactions = txs.len();
    let mut last_txs_len = sent_n_transactions;
    let mut last_sent_time = Instant::now();
    loop {
        ckb_testkit::info!(
            "waiting dispatch-transactions been committed, rest {}/{}",
            txs.len(),
            sent_n_transactions
        );

        txs = txs
            .into_iter()
            .filter(|tx| {
                let txstatus_opt = nodes[0].rpc_client().get_transaction(tx.hash());
                if let Some(txstatus) = txstatus_opt {
                    if txstatus.tx_status.status == Status::Committed {
                        return false;
                    }
                } else {
                    ckb_testkit::error!("tx {:#x} disappeared!", tx.hash());
                }
                true
            })
            .collect();

        if txs.is_empty() {
            break;
        } else if last_sent_time.elapsed() > Duration::from_secs(60) {
            if last_txs_len == txs.len() {
                txs.iter().for_each(|tx| {
                    let result = nodes[0]
                        .rpc_client()
                        .send_transaction_result(tx.data().into());
                    match result {
                        Ok(_) => {
                            ckb_testkit::info!("resend tx {:#x} success", tx.hash());
                        }
                        Err(err) => {
                            if !err.to_string().contains("Duplicated") {
                                ckb_testkit::error!(
                                    "failed to send tx {:#x}, error: {}",
                                    tx.hash(),
                                    err
                                );
                            }
                        }
                    }
                });
            }
            last_txs_len = txs.len();
            last_sent_time = Instant::now();
        } else {
            sleep(Duration::from_secs(1));
        }
    }

    assert!(
        i_out >= total_outs,
        "i_out: {}, total_outs: {}",
        i_out,
        total_outs
    );
    ckb_testkit::info!("finished dispatch");
}

pub fn collect(nodes: &[Node], owner: &User, users: &[User]) {
    ckb_testkit::info!("collect with params --n-users {}", users.len());
    let n_users = users.len();
    let mut last_logging_time = Instant::now();
    for (i_user, user) in users.iter().enumerate() {
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
                .lock(owner.single_secp256k1_lock_script_via_data())
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
            let result = maybe_retry_send_transaction(&nodes[0], &signed_tx);
            if last_logging_time.elapsed() > Duration::from_secs(30) {
                last_logging_time = Instant::now();
                ckb_testkit::info!("already collected {}/{} users", i_user, n_users)
            }
            assert!(
                result.is_ok(),
                "collect-transaction {:#x} should be ok but got {}",
                signed_tx.hash(),
                result.unwrap_err()
            );
        }
    }
    ckb_testkit::info!("finished collecting");
}

pub fn derive_privkeys(basic_raw_privkey: Byte32, n: usize) -> Vec<Privkey> {
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
