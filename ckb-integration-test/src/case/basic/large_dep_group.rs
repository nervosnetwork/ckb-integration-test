use crate::case::{Case, CaseOptions};
use crate::CKB2021;
use ckb_testkit::NodeOptions;
use ckb_testkit::Nodes;
use ckb_types::core::cell::CellMeta;
use ckb_types::core::{Capacity, DepType, TransactionBuilder, TransactionView};
use ckb_types::packed::{CellDep, CellDepBuilder, CellInput, CellOutput, OutPoint, OutPointVec};
use ckb_types::prelude::*;
use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

const N_DEP_GROUPS: usize = 1;
const N_TRANSACTIONS_PER_DEP_GROUP: usize = 100;

pub struct LargeDepGroup;

impl Case for LargeDepGroup {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![NodeOptions {
                node_name: "node2021",
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/Height1000002V2TestData",
                chain_spec: "testdata/spec/ckb2021",
                app_config: "testdata/config/ckb2021",
            }],
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");
        let live_cells = node2021.get_spendable_always_success_cells();

        {
            // warm up cache
            for live_cell in live_cells.iter().take(20000) {
                let _ = node2021
                    .rpc_client()
                    .get_live_cell(live_cell.out_point.clone().into(), false);
            }
        }

        let mut rng = thread_rng();
        let mut dep_groups = HashSet::new();
        let vec_n_out_points = vec![8, 32, 64, 72, 80, 88, 96, 104, 112, 120, 128, 136, 144, 256];

        for n_out_points in vec_n_out_points.clone() {
            for _ in 0..N_DEP_GROUPS {
                let output_data = {
                    let mut out_points_set = HashSet::new();
                    while out_points_set.len() < n_out_points {
                        let out_point = get_random_out_point(&mut rng, &live_cells);
                        out_points_set.insert(out_point);
                    }
                    let mut builder = OutPointVec::new_builder();
                    for out_point in out_points_set {
                        builder = builder.push(out_point);
                    }
                    builder.build().as_bytes().pack()
                };
                let output_data_capacity = Capacity::bytes(output_data.len()).unwrap();
                let output = CellOutput::new_builder()
                    .lock(node2021.always_success_script())
                    .build_exact_capacity(output_data_capacity)
                    .unwrap();
                let inputs = {
                    let mut inputs_capacity = 0;
                    let mut inputs_ = Vec::new();
                    while inputs_capacity < output.capacity().unpack() {
                        node2021.mine(1);
                        let txhash = node2021.get_tip_block().transaction(0).unwrap().hash();
                        let out_point = OutPoint::new(txhash, 0);
                        let cell_meta = node2021.get_cell_meta(out_point.clone());
                        let input = CellInput::new(out_point, 0);
                        inputs_capacity += cell_meta.capacity().as_u64();
                        inputs_.push(input);
                    }
                    inputs_
                };
                let cell_dep = node2021.always_success_cell_dep();
                let tx = TransactionBuilder::default()
                    .output(output)
                    .output_data(output_data)
                    .inputs(inputs)
                    .cell_dep(cell_dep)
                    .build();
                node2021.submit_transaction(&tx);

                let dep_group = CellDepBuilder::default()
                    .out_point(OutPoint::new(tx.hash(), 0))
                    .dep_type(DepType::DepGroup.into())
                    .build();
                dep_groups.insert((n_out_points, dep_group));
            }
        }

        loop {
            node2021.mine(10);
            let tx_pool_info = node2021.rpc_client().tx_pool_info();
            if tx_pool_info.total_tx_size.value() == 0 {
                break;
            }
        }

        {
            let mut n_deps_txs: HashMap<usize, Vec<TransactionView>> = HashMap::new();
            for (n_out_points, cell_dep) in dep_groups {
                for _ in 0..N_TRANSACTIONS_PER_DEP_GROUP {
                    let input = {
                        node2021.mine(1);
                        let txhash = node2021.get_tip_block().transaction(0).unwrap().hash();
                        let out_point = OutPoint::new(txhash, 0);
                        node2021.get_cell_meta(out_point.clone())
                    };
                    let tx = node2021
                        .always_success_transaction(&input)
                        .as_advanced_builder()
                        .cell_dep(cell_dep.clone())
                        .build();
                    n_deps_txs.entry(n_out_points).or_default().push(tx);
                }
            }
            ckb_testkit::info!("const N_DEP_GROUPS = {}", N_DEP_GROUPS);
            ckb_testkit::info!(
                "const N_TRANSACTIONS_PER_DEP_GROUP = {}",
                N_TRANSACTIONS_PER_DEP_GROUP
            );
            for n_deps in vec_n_out_points.clone() {
                let txs = n_deps_txs.get(&n_deps).unwrap();
                let start_time = Instant::now();
                for tx in txs.iter() {
                    node2021.submit_transaction(tx);
                }
                let elapsed = start_time.elapsed();
                ckb_testkit::info!(
                    "send {}({} * {}) txs with {:5}-dep-groups, elapsed: {}ms",
                    txs.len(),
                    N_DEP_GROUPS,
                    N_TRANSACTIONS_PER_DEP_GROUP,
                    n_deps,
                    elapsed.as_millis()
                );
            }
        }
    }
}

fn get_random_out_point<R: Rng + ?Sized>(rng: &mut R, live_cells: &[CellMeta]) -> OutPoint {
    let x: usize = rng.gen_range(0..live_cells.len());
    live_cells[x].out_point.clone()
}
