use crate::case::{Case, CaseOptions};
use crate::CKB2021;
use ckb_testkit::NodeOptions;
use ckb_testkit::Nodes;
use ckb_types::core::{Capacity, DepType, TransactionBuilder, TransactionView};
use ckb_types::packed::{CellDep, CellDepBuilder, CellInput, CellOutput, OutPoint, OutPointVec};
use ckb_types::prelude::*;
use std::collections::HashMap;
use std::time::Instant;

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
        let mut dep_groups: HashMap<usize, CellDep> = HashMap::new();
        let n_deps_vec = vec![1usize, 10, 100, 1000, 10000];

        {
            let mut live_cells = node2021.get_spendable_always_success_cells();
            for n_deps in n_deps_vec.clone() {
                let output_data = {
                    let mut builder = OutPointVec::new_builder();
                    for _ in 0..n_deps {
                        builder = builder.push(live_cells.pop().unwrap().out_point);
                    }
                    builder.build().as_bytes().pack()
                };
                let output_data_capacity = Capacity::bytes(output_data.len()).unwrap();
                let output = CellOutput::new_builder()
                    .lock(node2021.always_success_script())
                    .build_exact_capacity(output_data_capacity)
                    .unwrap();
                let inputs = {
                    let mut inputs_ = Vec::new();
                    let mut inputs_capacity_ = 0;
                    loop {
                        let input = live_cells.pop().unwrap();
                        inputs_capacity_ += input.capacity().as_u64();
                        inputs_.push(input);
                        if inputs_capacity_ >= output.capacity().unpack() {
                            break inputs_
                                .iter()
                                .map(|input| CellInput::new(input.out_point.clone(), 0))
                                .collect::<Vec<_>>();
                        }
                    }
                };
                let cell_dep = node2021.always_success_cell_dep();
                let tx = TransactionBuilder::default()
                    .inputs(inputs)
                    .output(output)
                    .output_data(output_data)
                    .cell_dep(cell_dep)
                    .build();
                node2021.submit_transaction(&tx);

                let dep_group = CellDepBuilder::default()
                    .out_point(OutPoint::new(tx.hash(), 0))
                    .dep_type(DepType::DepGroup.into())
                    .build();
                dep_groups.insert(n_deps, dep_group);
            }
        }

        node2021.mine(10);

        {
            let mut live_cells = node2021.get_spendable_always_success_cells();
            let mut n_deps_txs: HashMap<usize, Vec<TransactionView>> = HashMap::new();
            for n_deps in n_deps_vec.clone() {
                for _ in 0..100 {
                    let cell_dep = dep_groups.get(&n_deps).unwrap();
                    let input = live_cells.pop().unwrap();
                    let tx = node2021
                        .always_success_transaction(&input)
                        .as_advanced_builder()
                        .cell_dep(cell_dep.clone())
                        .build();
                    n_deps_txs.entry(n_deps).or_default().push(tx);
                }
            }
            for n_deps in n_deps_vec.clone() {
                let txs = n_deps_txs.get(&n_deps).unwrap();
                let start_time = Instant::now();
                for tx in txs.iter() {
                    node2021.submit_transaction(tx);
                }
                let elapsed = start_time.elapsed();
                ckb_testkit::info!(
                    "send {} txs with {}-dep-groups, elapsed: {}ms",
                    txs.len(),
                    n_deps,
                    elapsed.as_millis()
                );
            }
        }
    }
}
