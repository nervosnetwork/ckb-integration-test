use ckb_testkit::{Node, User};
use ckb_types::core::{TransactionBuilder, TransactionView};
use ckb_types::packed::{CellDep, CellOutput};
use ckb_types::{
    core::cell::CellMeta,
    packed::{Byte32, CellInput, OutPoint},
    prelude::*,
};
use crossbeam_channel::{Receiver, Sender};
use lru::LruCache;
use std::collections::HashMap;
use std::time::Instant;

pub struct LiveCellProducer {
    users: Vec<User>,
    nodes: Vec<Node>,
    seen_out_points: LruCache<OutPoint, Instant>,
}

impl LiveCellProducer {
    pub fn new(users: Vec<User>, nodes: Vec<Node>) -> Self {
        let n_users = users.len();
        Self {
            users,
            nodes,
            seen_out_points: LruCache::new(n_users + 10),
        }
    }

    pub fn run(mut self, live_cell_sender: Sender<CellMeta>) {
        loop {
            // FIXME better use Nodes::get_fix_header()
            let min_tip_number = self
                .nodes
                .iter()
                .map(|node| node.get_tip_block_number())
                .min()
                .unwrap();
            for user in self.users.iter() {
                let live_cells = user
                    .get_spendable_single_secp256k1_cells(&self.nodes[0])
                    .into_iter()
                    // TODO reduce competition
                    .filter(|cell| {
                        if self.seen_out_points.contains(&cell.out_point) {
                            return false;
                        }
                        let tx_info = cell
                            .transaction_info
                            .as_ref()
                            .expect("live cell's transaction info should be ok");
                        if tx_info.block_number < min_tip_number {
                            return false;
                        }
                        true
                    })
                    .collect::<Vec<_>>();
                for cell in live_cells {
                    self.seen_out_points
                        .put(cell.out_point.clone(), Instant::now());
                    let _ignore = live_cell_sender.send(cell);
                }
            }
        }
    }
}

pub struct TransactionProducer {
    // #{ lock_hash => user }
    users: HashMap<Byte32, User>,
    cell_deps: Vec<CellDep>,
    n_outputs: usize,
    // #{ lock_hash => live_cell }
    live_cells: HashMap<Byte32, CellMeta>,
    // #{ out_point => live_cell }
    backlogs: HashMap<OutPoint, CellMeta>,
}

impl TransactionProducer {
    pub fn new(users: Vec<User>, cell_deps: Vec<CellDep>, n_outputs: usize) -> Self {
        let users = users
            .into_iter()
            .map(|user| (user.single_secp256k1_lock_hash(), user))
            .collect();
        Self {
            users,
            cell_deps,
            n_outputs,
            live_cells: HashMap::new(),
            backlogs: HashMap::new(),
        }
    }

    pub fn run(
        mut self,
        live_cell_receiver: Receiver<CellMeta>,
        transaction_sender: Sender<TransactionView>,
    ) {
        while let Ok(live_cell) = live_cell_receiver.recv() {
            let lock_hash = live_cell.cell_output.calc_lock_hash();
            match self.live_cells.entry(lock_hash) {
                std::collections::hash_map::Entry::Occupied(entry) => {
                    if entry.get().out_point == live_cell.out_point {
                        self.backlogs.insert(live_cell.out_point.clone(), live_cell);
                    }
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(live_cell);
                }
            }

            if self.live_cells.len() >= self.n_outputs {
                let mut live_cells = HashMap::new();
                std::mem::swap(&mut self.live_cells, &mut live_cells);

                let inputs = live_cells.values().map(|cell| {
                    CellInput::new_builder()
                        .previous_output(cell.out_point.clone())
                        .build()
                });
                let outputs = live_cells.values().map(|cell| {
                    CellOutput::new_builder()
                        .capacity((cell.capacity().as_u64() - 1).pack())
                        .lock(cell.cell_output.lock())
                        .build()
                });
                let outputs_data = live_cells.values().map(|_| Default::default());
                let raw_tx = TransactionBuilder::default()
                    .inputs(inputs)
                    .outputs(outputs)
                    .outputs_data(outputs_data)
                    .cell_deps(self.cell_deps.clone())
                    .build();
                let witnesses = live_cells.values().map(|cell| {
                    let lock_hash = cell.cell_output.calc_lock_hash();
                    let user = self.users.get(&lock_hash).expect("should be ok");
                    user.single_secp256k1_signed_witness(&raw_tx)
                        .as_bytes()
                        .pack()
                });
                let signed_tx = raw_tx.as_advanced_builder().witnesses(witnesses).build();

                if transaction_sender.send(signed_tx).is_err() {
                    // SendError occurs, the corresponding transaction receiver is dead
                    return;
                }

                let mut backlogs = HashMap::new();
                std::mem::swap(&mut self.backlogs, &mut backlogs);
                for (_, live_cell) in backlogs.into_iter() {
                    let lock_hash = live_cell.cell_output.calc_lock_hash();
                    match self.live_cells.entry(lock_hash) {
                        std::collections::hash_map::Entry::Occupied(entry) => {
                            if entry.get().out_point == live_cell.out_point {
                                self.backlogs.insert(live_cell.out_point.clone(), live_cell);
                            }
                        }
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            entry.insert(live_cell);
                        }
                    }
                }
            }
        }
    }
}
