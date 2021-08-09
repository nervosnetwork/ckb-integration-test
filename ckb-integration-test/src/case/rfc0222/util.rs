use ckb_testkit::Node;
use ckb_types::core::{Capacity, TransactionBuilder, TransactionView};
use ckb_types::packed::{Bytes, CellDep, CellInput, CellOutput, OutPoint, Script};
use ckb_types::prelude::*;

pub(super) fn build_transaction(
    node: &Node,
    type_: Option<Script>,
    cell_deps: Vec<CellDep>,
) -> TransactionView {
    let input = node.get_spendable_always_success_cells()[0].to_owned();
    TransactionBuilder::default()
        .input(CellInput::new(input.out_point.clone(), 0))
        .output(
            CellOutput::new_builder()
                .lock(input.cell_output.lock())
                .type_(type_.pack())
                .capacity(input.capacity().pack())
                .build(),
        )
        .output_data(Default::default())
        .set_cell_deps(cell_deps)
        .build()
}

#[derive(Default)]
pub(super) struct RFC0222CellDeployer {
    always_success_cell_dep_a1: Option<CellDep>,
    always_success_cell_dep_a2: Option<CellDep>,
    always_success_cell_dep_b1: Option<CellDep>,
}

impl RFC0222CellDeployer {
    pub(super) fn deploy(&mut self, node2021: &Node) {
        // Deploy our data cells onto chain.
        let always_success_cell_dep_a1 = {
            let output_data = include_bytes!("../../../testdata/spec/ckb2021/cells/always_success");
            let type_ = node2021.always_success_script();
            let out_point = Self::deploy_cell_with_type_(node2021, output_data.pack(), type_);
            CellDep::new_builder().out_point(out_point).build()
        };
        let always_success_cell_dep_a2 = {
            let output_data = include_bytes!("../../../testdata/spec/ckb2021/cells/always_success");
            let type_ = node2021.always_success_script();
            let out_point = Self::deploy_cell_with_type_(node2021, output_data.pack(), type_);
            CellDep::new_builder().out_point(out_point).build()
        };
        let always_success_cell_dep_b1 = {
            let output_data =
                include_bytes!("../../../testdata/spec/ckb2021/cells/another_always_success");
            let type_ = node2021.always_success_script();
            let out_point = Self::deploy_cell_with_type_(node2021, output_data.pack(), type_);
            CellDep::new_builder().out_point(out_point).build()
        };
        self.always_success_cell_dep_a1 = Some(always_success_cell_dep_a1);
        self.always_success_cell_dep_a2 = Some(always_success_cell_dep_a2);
        self.always_success_cell_dep_b1 = Some(always_success_cell_dep_b1);
    }

    fn deploy_cell_with_type_(node: &Node, output_data: Bytes, type_: Script) -> OutPoint {
        let mut output_data_capacity = Capacity::bytes(output_data.len())
            .expect("calc capacity for output data")
            .as_u64();
        let mut inputs_capacity = 0;
        let mut inputs = Vec::new();
        for cell in node.get_spendable_always_success_cells() {
            let capacity: u64 = cell.cell_output.capacity().unpack();
            if output_data_capacity >= capacity {
                output_data_capacity -= capacity;
                inputs_capacity += capacity;
                inputs.push(cell);
            } else {
                inputs_capacity += capacity;
                inputs.push(cell);
                break;
            }
        }
        let tx = TransactionBuilder::default()
            .inputs(
                inputs
                    .into_iter()
                    .map(|input| CellInput::new(input.out_point, 0)),
            )
            .output(
                CellOutput::new_builder()
                    .lock(node.always_success_script())
                    .type_(Some(type_).pack())
                    .capacity(inputs_capacity.pack())
                    .build(),
            )
            .output_data(output_data)
            .cell_dep(node.always_success_cell_dep())
            .build();

        // Submit transaction and mine until it be committed
        node.submit_transaction(&tx);
        node.mine(node.consensus().tx_proposal_window.closest.value() + 1);
        assert!(
            node.is_transaction_committed(&tx),
            "transaction should be committed, but got {:?}",
            node.rpc_client().get_transaction(tx.hash()),
        );

        OutPoint::new(tx.hash(), 0)
    }

    pub(super) fn always_success_cell_dep_a1(&self) -> CellDep {
        self.always_success_cell_dep_a1.clone().unwrap()
    }

    pub(super) fn always_success_cell_dep_a2(&self) -> CellDep {
        self.always_success_cell_dep_a2.clone().unwrap()
    }

    pub(super) fn always_success_cell_dep_b1(&self) -> CellDep {
        self.always_success_cell_dep_b1.clone().unwrap()
    }
}
