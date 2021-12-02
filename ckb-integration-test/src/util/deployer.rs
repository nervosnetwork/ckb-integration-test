use ckb_testkit::ckb_types::core::cell::CellMeta;
use ckb_testkit::ckb_types::core::TransactionBuilder;
use ckb_testkit::ckb_types::packed::{Bytes, CellInput, CellOutput, OutPoint};
use ckb_testkit::ckb_types::prelude::*;
use ckb_testkit::{BuildInstruction, Node};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Deployer {
    // #{ name => cell-meta }
    deployed_cells: HashMap<String, CellMeta>,
}

impl Deployer {
    pub fn new() -> Deployer {
        Default::default()
    }

    pub fn deploy<S: ToString>(
        &mut self,
        node: &Node,
        cell_name: S,
        output: CellOutput,
        output_data: Bytes,
    ) {
        let cell_name = cell_name.to_string();
        ckb_testkit::debug!(
            "[Node {}] deploying cell \"{}\"",
            node.node_name(),
            cell_name
        );
        assert!(
            !self.deployed_cells.contains_key(&cell_name.to_string()),
            "cell \"{}\" already deployed",
            cell_name,
        );

        // Pick inputs
        let mut output_capacity: u64 = output.capacity().unpack();
        let mut inputs = Vec::new();
        for cell in node.get_spendable_always_success_cells() {
            let capacity: u64 = cell.cell_output.capacity().unpack();
            if output_capacity >= capacity {
                output_capacity -= capacity;
                inputs.push(cell);
            } else {
                inputs.push(cell);
                break;
            }
        }

        // Construct transaction
        let tx = TransactionBuilder::default()
            .inputs(
                inputs
                    .into_iter()
                    .map(|input| CellInput::new(input.out_point, 0)),
            )
            .output(output)
            .output_data(output_data)
            .cell_dep(node.always_success_cell_dep())
            .build();

        // Make sure transaction committed
        let tip_number = node.get_tip_block_number();
        node.build_according_to_instructions(
            tip_number + 3,
            vec![
                BuildInstruction::Propose {
                    template_number: tip_number + 1,
                    proposal_short_id: tx.proposal_short_id(),
                },
                BuildInstruction::Commit {
                    template_number: tip_number + 3,
                    transaction: tx.clone(),
                },
            ],
        )
        .unwrap_or_else(|err| panic!("failed to deploy \"{}\", error: {}", cell_name, err));

        // Save cell-meta inside deployer
        {
            let _ = node.indexer();
        }
        let out_point = OutPoint::new(tx.hash(), 0);
        let cell_meta = node.get_cell_meta(out_point).expect(&format!(
            "deployer should already committed tx {:#x}",
            tx.hash()
        ));
        self.deployed_cells.insert(cell_name, cell_meta);
    }

    pub fn get_out_point<S: ToString>(&self, cell_name: S) -> OutPoint {
        self.get_cell(cell_name).out_point
    }

    pub fn get_cell<S: ToString>(&self, cell_name: S) -> CellMeta {
        let cell_name = cell_name.to_string();
        self.deployed_cells
            .get(&cell_name)
            .expect(&format!("deployer cannot find cell {}", cell_name))
            .clone()
    }

    pub fn get_cells(&self) -> HashMap<String, CellMeta> {
        self.deployed_cells.clone()
    }
}
