use crate::case::rfc0222::util::{build_transaction, deploy_cell_with_type_};
use crate::case::{Case, CaseOptions};
use crate::node::{Node, NodeOptions};
use crate::nodes::Nodes;
use crate::{CKB2019, CKB2021};
use ckb_types::{
    core::{EpochNumber, ScriptHashType},
    packed::{CellDep, Script},
    prelude::*,
};

const RFC0222_EPOCH_NUMBER: EpochNumber = 3;
const ERROR_MULTIPLE_MATCHES: &str = "MultipleMatches";

pub struct RFC0222BeforeSwitch;

impl Case for RFC0222BeforeSwitch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: true,
            make_all_nodes_synced: true,
            make_all_nodes_connected_and_synced: true,
            node_options: vec![
                NodeOptions {
                    node_name: "node2019",
                    ckb_binary: CKB2019.read().unwrap().clone(),
                    initial_database: "db/empty",
                    chain_spec: "spec/multiple-always-success",
                    app_config: "config/ckb2021",
                },
                NodeOptions {
                    node_name: "node2021",
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "db/Epoch2V2TestData",
                    chain_spec: "spec/multiple-always-success",
                    app_config: "config/ckb2021",
                },
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2019 = nodes.get_node("node2019");
        let node2021 = nodes.get_node("node2021");
        assert!(!is_rfc0222_switched(node2021));

        // Deploy our data cells onto chain.
        let always_success_cell_dep_a1 = {
            let output_data = include_bytes!(
                "../../../testdata/spec/multiple-always-success/cells/always_success"
            );
            let type_ = node2021.always_success_script();
            let out_point = deploy_cell_with_type_(node2021, output_data.pack(), type_);
            CellDep::new_builder().out_point(out_point).build()
        };
        let always_success_cell_dep_a2 = {
            let output_data = include_bytes!(
                "../../../testdata/spec/multiple-always-success/cells/always_success"
            );
            let type_ = node2021.always_success_script();
            let out_point = deploy_cell_with_type_(node2021, output_data.pack(), type_);
            CellDep::new_builder().out_point(out_point).build()
        };
        let always_success_cell_dep_b1 = {
            let output_data = include_bytes!(
                "../../../testdata/spec/multiple-always-success/cells/another_always_success"
            );
            let type_ = node2021.always_success_script();
            let out_point = deploy_cell_with_type_(node2021, output_data.pack(), type_);
            CellDep::new_builder().out_point(out_point).build()
        };

        let cases = vec![
            // case-0
            (None, vec![always_success_cell_dep_a1.clone()], Ok(())),
            // case-1
            (
                // node2021.always_success_script() references by `ScriptHashType::Data`
                None,
                vec![
                    node2021.always_success_cell_dep(),
                    always_success_cell_dep_a1.clone(),
                    always_success_cell_dep_a2.clone(),
                    always_success_cell_dep_b1.clone(),
                ],
                Ok(()),
            ),
            // case-2
            (
                Some(
                    // Only match always_success_cell_dep_a1
                    Script::new_builder()
                        .code_hash(node2021.always_success_script().calc_script_hash())
                        .hash_type(ScriptHashType::Type.into())
                        .build(),
                ),
                vec![
                    node2021.always_success_cell_dep(),
                    always_success_cell_dep_a1.clone(),
                ],
                Ok(()),
            ),
            // case-3
            (
                Some(
                    // match to always_success_cell_dep_a1 and always_success_cell_dep_a2,
                    // always_success_cell_dep_a1 and always_success_cell_dep_a2 have the same data hash
                    Script::new_builder()
                        .code_hash(node2021.always_success_script().calc_script_hash())
                        .hash_type(ScriptHashType::Type.into())
                        .build(),
                ),
                vec![
                    node2021.always_success_cell_dep(),
                    always_success_cell_dep_a1.clone(),
                    always_success_cell_dep_a2.clone(),
                ],
                Err(ERROR_MULTIPLE_MATCHES),
            ),
            // case-4
            (
                Some(
                    // match to always_success_cell_dep_a1 and always_success_cell_dep_b1,
                    // always_success_cell_dep_a1 and always_success_cell_dep_b1 have the different data hash
                    Script::new_builder()
                        .code_hash(node2021.always_success_script().calc_script_hash())
                        .hash_type(ScriptHashType::Type.into())
                        .build(),
                ),
                vec![
                    node2021.always_success_cell_dep(),
                    always_success_cell_dep_a1.clone(),
                    always_success_cell_dep_b1.clone(),
                ],
                Err(ERROR_MULTIPLE_MATCHES),
            ),
        ];
        for (i, (type_, cell_deps, expected)) in cases.into_iter().enumerate() {
            assert!(!is_rfc0222_switched(node2021));
            nodes.waiting_for_sync().expect("nodes should be synced");

            let tx = build_transaction(node2021, type_, cell_deps);
            let actual = node2021
                .rpc_client()
                .send_transaction_result(tx.pack().data().into());
            let actual2019 = node2019
                .rpc_client()
                .send_transaction_result(tx.pack().data().into());
            assert_eq!(
                actual.as_ref().map_err(|err| err.to_string()),
                actual2019.as_ref().map_err(|err| err.to_string()),
                "case-{} expect both node2021 and node2019 return the same",
                i,
            );
            match (expected, actual) {
                (Ok(()), Ok(_)) => {}
                (Err(errmsg), Err(err)) => {
                    assert!(
                        err.to_string().contains(errmsg),
                        "case-{} expect Err(\".*{}.*\"), but got Err({:?})",
                        i,
                        errmsg,
                        err
                    );
                }
                (Ok(()), Err(err)) => {
                    panic!("case-{} expect Ok(()), but got: Err({:?})", i, err)
                }
                (Err(errmsg), Ok(block_hash)) => {
                    panic!(
                        "case-{} expect Err(\".*{}.*\"), but got: Ok({:#x})",
                        i, errmsg, block_hash
                    )
                }
            }
        }
    }
}

fn is_rfc0222_switched(node: &Node) -> bool {
    node.rpc_client().get_current_epoch().number.value() >= RFC0222_EPOCH_NUMBER
}
