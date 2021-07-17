use crate::case::rfc0222::util::{build_transaction, RFC0222CellDeployer};
use crate::case::{Case, CaseOptions};
use crate::util::calc_epoch_start_number;
use crate::CKB2021;
use ckb_testkit::{NodeOptions, Nodes};
use ckb_types::{
    core::{EpochNumber, ScriptHashType},
    packed::Script,
    prelude::*,
};

const RFC0222_EPOCH_NUMBER: EpochNumber = 3;
const ERROR_MULTIPLE_MATCHES: &str = "MultipleMatches";

pub struct RFC0222AfterSwitch;

impl Case for RFC0222AfterSwitch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: true,
            make_all_nodes_synced: true,
            make_all_nodes_connected_and_synced: true,
            node_options: vec![NodeOptions {
                node_name: "node2021",
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/Epoch2V2TestData",
                chain_spec: "testdata/spec/ckb2021",
                app_config: "testdata/config/ckb2021",
            }]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");

        // Deploy our data cells onto chain.
        let mut deployer = RFC0222CellDeployer::default();
        deployer.deploy(node2021);

        node2021.mine_to(calc_epoch_start_number(node2021, RFC0222_EPOCH_NUMBER));
        let cases = vec![
            // case-0
            (None, vec![node2021.always_success_cell_dep()], Ok(())),
            // case-1
            (
                // node2021.always_success_script() references by `ScriptHashType::Data`
                None,
                vec![
                    node2021.always_success_cell_dep(),
                    deployer.always_success_cell_dep_a1(),
                    deployer.always_success_cell_dep_a2(),
                    deployer.always_success_cell_dep_b1(),
                ],
                Ok(()),
            ),
            // case-2
            (
                Some(
                    // Only match to always_success_cell_dep_a1
                    Script::new_builder()
                        .code_hash(node2021.always_success_script().calc_script_hash())
                        .hash_type(ScriptHashType::Type.into())
                        .build(),
                ),
                vec![
                    node2021.always_success_cell_dep(),
                    deployer.always_success_cell_dep_a1(), // always_success_cell_dep_a1.clone(), DuplicateCellDeps
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
                    deployer.always_success_cell_dep_a1(),
                    deployer.always_success_cell_dep_a2(),
                ],
                Ok(()),
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
                    deployer.always_success_cell_dep_a1(),
                    deployer.always_success_cell_dep_b1(),
                ],
                Err(ERROR_MULTIPLE_MATCHES),
            ),
        ];
        for (i, (type_, cell_deps, expected)) in cases.into_iter().enumerate() {
            let tx = build_transaction(node2021, type_, cell_deps);
            let actual = node2021
                .rpc_client()
                .send_transaction_result(tx.pack().data().into());
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
