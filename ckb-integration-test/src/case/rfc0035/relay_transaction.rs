use super::RFC0035_EPOCH_NUMBER;
use crate::{
    preclude::*,
    util::{estimate_start_number_of_epoch, v0_100, v0_43, },
};
use ckb_testkit::connector::{
    ConnectorBuilder,Connector, SimpleServiceHandler, SimpleProtocolHandler, SharedState
};
use ckb_testkit::ckb_jsonrpc_types::Consensus;
use ckb_testkit::SupportProtocols;
use ckb_testkit::util::wait_until;
use ckb_testkit::SYSTEM_CELL_ALWAYS_SUCCESS_INDEX;
use ckb_testkit::ckb_types::{
    core::{
        cell::CellMeta, BlockNumber, Cycle, ScriptHashType, TransactionBuilder, TransactionView,
    },
    packed::{CellInput, CellOutput, OutPoint, RelayMessageReader, Script},
    prelude::*,
};
use std::time::Duration;

/// ## `RFC0035RelayTransaction`
///                                                                                                 
/// RFC0035 introduces a change in how a node handles `RelayTransactions`.
///                                                                                                 
/// We want to make sure that v0.100 nodes behave as expected. Let's first think about what factors may affect *RelayTransactions*.
///                                                                                                 
///   1. Whether the v0.100 activates RFC0035: `tip_number <= 2999` indicates non-activated,
///      `tip_number >= 2999` indicates activated.
///      - 2998
///      - 2999
///                                                                                                 
///   2. The peer's client version, v0.100 use different logic
///      for nodes with different client versions:
///      - v0.43
///      - v0.100
///                                                                                                 
///   3. The underlying network protocol to relay transaction:
///     - relay
///     - relay_v2
///                                                                                                 
///   4. The relaying transaction's script `hash_type`:
///     - Data
///     - Data1
///     - Type
///                                                                                                 
///   5. The attached `cycles` within `RelayTransaction` message:
///     - vm0-cycles
///     - vm1-cycles
///     - 0 // TODO
///                                                                                                 
/// Next, we observe the node's behaviors via RPC `get_transaction` to check whether the node accepts the transaction.
///                                                                                                 
/// ## Cases
///                                                                                                 
/// 1. Start a testing node and make it grow up to `node tip`.
/// 2. Start a version-specified(`peer version`) network connector and connect to the above node.
/// 3. Construct a transaction with specified `tx.type_.hash_type`.
/// 4. Network connector relays the above transaction, attached with specified `relayed cycles`, to
///    the testing node.
/// 5. Observe the transaction status and banned status.
///                                                                                                 
/// ```text
/// ┌─────┬──────┬─────────┬──────────┬───────────┬──────────────┬────────────────────────────────
/// │ id  │ node │ peer    │ network  │ tx.script │ relayed      │ result                         │
/// │     │ tip  │ version │ protocol │ hash_type │ cycles       │                                │
/// └─────┴──────┴─────────┴──────────┴───────────┴──────────────┴────────────────────────────────
/// │ 1   │ 2998 │ v0.43   │ relay    │ data      │ vm0-cycles   │ Ok(())                         │
/// │ 2   │ 2998 │ v0.43   │ relay    │ data      │ vm1-cycles   │ Err(RelayTransactionFailed)    │
/// │ 3   │ 2998 │ v0.43   │ relay    │ type      │ vm0-cycles   │ Ok(())                         │
/// │ 4   │ 2998 │ v0.43   │ relay    │ type      │ vm1-cycles   │ Err(RelayTransactionFailed)    │
/// │ 5   │ 2998 │ v0.43   │ relay    │ data1     │ vm0-cycles   │ Err(RelayTransactionFailed)    │
/// │ 6   │ 2998 │ v0.43   │ relay    │ data1     │ vm1-cycles   │ Err(RelayTransactionFailed)    │
/// │ 7   │ 2998 │ v0.43   │ relay_v2 │ data      │ vm0-cycles   │ Err(RelayTransactionHashFailed)│
/// │ 8   │ 2998 │ v0.100  │ relay    │ data      │ vm0-cycles   │ Ok(())                         │
/// │ 9   │ 2998 │ v0.100  │ relay    │ data      │ vm1-cycles   │ Err(RelayTransactionFailed)    │
/// │ 10  │ 2998 │ v0.100  │ relay    │ type      │ vm0-cycles   │ Ok(())                         │
/// │ 11  │ 2998 │ v0.100  │ relay    │ type      │ vm1-cycles   │ Err(RelayTransactionFailed)    │
/// │ 12  │ 2998 │ v0.100  │ relay    │ data1     │ vm0-cycles   │ Err(RelayTransactionFailed)    │
/// │ 13  │ 2998 │ v0.100  │ relay    │ data1     │ vm1-cycles   │ Err(RelayTransactionFailed)    │
/// │ 14  │ 2998 │ v0.100  │ relay_v2 │ data      │ vm0-cycles   │ Err(RelayTransactionHashFailed)│
/// │ 15  │ 2999 │ v0.43   │ relay_v2 │ data      │ vm0-cycles   │ Ok(())                         │
/// │ 16  │ 2999 │ v0.43   │ relay_v2 │ data      │ vm1-cycles   │ Err(RelayTransactionFailed)    │
/// │ 17  │ 2999 │ v0.43   │ relay_v2 │ type      │ vm0-cycles   │ Err(RelayTransactionFailed)    │
/// │ 18  │ 2999 │ v0.43   │ relay_v2 │ type      │ vm1-cycles   │ Ok(())                         │
/// │ 19  │ 2999 │ v0.43   │ relay_v2 │ data1     │ vm0-cycles   │ Err(RelayTransactionFailed)    │
/// │ 20  │ 2999 │ v0.43   │ relay_v2 │ data1     │ vm1-cycles   │ Ok(())                         │
/// │ 21  │ 2999 │ v0.100  │ relay    │ data      │ vm0-cycles   │ Err(RelayTransactionHashFailed)│
/// │ 22  │ 2999 │ v0.100  │ relay_v2 │ data      │ vm0-cycles   │ Ok(())                         │
/// │ 23  │ 2999 │ v0.100  │ relay_v2 │ data      │ vm1-cycles   │ Err(RelayTransactionFailed)    │
/// │ 24  │ 2999 │ v0.100  │ relay_v2 │ type      │ vm0-cycles   │ Err(RelayTransactionFailed)    │
/// │ 25  │ 2999 │ v0.100  │ relay_v2 │ type      │ vm1-cycles   │ Ok(())                         │
/// │ 26  │ 2999 │ v0.100  │ relay_v2 │ data1     │ vm0-cycles   │ Err(RelayTransactionFailed)    │
/// │ 27  │ 2999 │ v0.100  │ relay_v2 │ data1     │ vm1-cycles   │ Ok(())                         │
/// └─────┴──────┴─────────┴──────────┴───────────┴──────────────┴────────────────────────────────
/// ```
///                                                                                                 
/// ## Failure explaining
///                                                                                                 
/// * RelayTransactionHashFailed
///                                                                                                 
///   Send `RelayTransactionHashes` to CKB node, but CKB node doesn't send us
///   `GetTransactions` message.
///                                                                                                 
///   It is because:
///     - fork2021-activated node discards receiving messages from RelayV2 protocol
///     - fork2021-non-activated node discards receiving messages from Relay protocol
///                                                                                                 
/// * RelayTransactionFailed
///                                                                                                 
///   Send `RelayTransactions` to CKB node, but CKB node doesn't accept that transaction.
///                                                                                                 
///   It is because the attached cycles is not matched with local execution result.

pub struct RFC0035RelayTransaction;

impl Case for RFC0035RelayTransaction {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![NodeOptions {
                node_name: String::from("node2021"),
                ckb_binary: CKB2021.read().unwrap().clone(),
                initial_database: "testdata/db/Epoch2V2TestData",
                chain_spec: "testdata/spec/ckb2021",
                // Note: `input.script.hash_type == "data"` ensures the input's scripts are
                // cycles-consistency
                app_config: "testdata/config/ckb2021_block_assembler_hash_type_is_data",
            }],
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");
        node2021.mine(13);

        let fork_switch_height = estimate_start_number_of_epoch(node2021, RFC0035_EPOCH_NUMBER);

        // Prepare a cycles-consystency input
        let input = {
            // Note: do not use `node2021.get_spendable_always_success_cells()`
            // as inputs cause `fn get_spendable_always_success_cells` indexes
            // cells via always-success-script with `hash_type: Type`
            let tip_block = node2021.get_tip_block();
            let tip_cellbase = tip_block.transaction(0).unwrap();
            let tip_cellbase_output_cell_meta = {
                node2021.indexer();
                node2021
                    .get_cell_meta(OutPoint::new(tip_cellbase.hash(), 0))
                    .unwrap()
            };
            tip_cellbase_output_cell_meta
        };
        assert!(
            input.cell_output.lock().hash_type() == ScriptHashType::Data.into(),
            "Ensure the input's scripts are cycles-consistency",
        );
        assert!(
            input.transaction_info.as_ref().unwrap().block_number < fork_switch_height - 3,
            "Ensure the input is valid for all cases scenarioes"
        );

        // Setup
        //
        // Prepare 3 kinds of transactions:
        //   - data0_tx, data0_tx.type_.hash_type = ScriptHashType::Data
        //   - data1_tx, data1_tx.type_.hash_type = ScriptHashType::Data1
        //   - type_tx,   type_tx.type_.hash_type = ScriptHashType::Type
        //
        // Calculate transaction cycles consumed when runs on VM0 and VM1:
        //   - vm0_cycles, transaction cycles consumed when runs on VM0
        //   - vm1_cycles, transaction cycles consumed when runs on VM1
        //
        // Node versions:
        //   - v0_43, `$CKB2019 --version`
        //   - v0_100, `$CKB2021 --version`
        let data0_tx;
        let data1_tx;
        let type_tx;
        let vm0_cycles;
        let vm1_cycles;
        {
            let node_used_to_dry_run_txs = {
                let node = node2021.clone_node("used_to_dry_run_txs");
                // Let `node_used_to_dry_run_txs` activates fork2021, so that
                // it allows data1-transactions
                node.pull_node(node2021).unwrap();
                node.mine_to(fork_switch_height);
                node
            };

            data0_tx = Self::build_transaction(node2021, &input, ScriptHashType::Data);
            data1_tx = Self::build_transaction(node2021, &input, ScriptHashType::Data1);
            type_tx = Self::build_transaction(node2021, &input, ScriptHashType::Type);
            vm0_cycles = node_used_to_dry_run_txs.get_transaction_cycles(&data0_tx);
            vm1_cycles = node_used_to_dry_run_txs.get_transaction_cycles(&data1_tx);
        };

        let cases = Self::filter_cases_params(fork_switch_height);
        for case in cases {
            let tx = match case.tx_script_hash_type {
                ScriptHashType::Data => data0_tx.clone(),
                ScriptHashType::Type => type_tx.clone(),
                ScriptHashType::Data1 => data1_tx.clone(),
            };
            let relayed_cycles = match case.relayed_cycles {
                ScriptHashType::Data => vm0_cycles,
                ScriptHashType::Type => unreachable!(),
                ScriptHashType::Data1 => vm1_cycles,
            };
            let node = self.setup_node(&case, node2021);
            let mut connector = self.setup_connector(&case, node.consensus());
            let actual_result = self.run(&case, &mut connector, &node, &tx, relayed_cycles);

            // {
            //     ckb_testkit::info!(
            //         "case_discription │ {:<3} │ {} │ {:<7} │ {:<8} │ {:<9} │ {:<12} │ {:?}",
            //         case.id,
            //         case.node_tip,
            //         {
            //             if case.peer_version == v0_100() {
            //                 "v0.100"
            //             } else {
            //                 "v0.43"
            //             }
            //         },
            //         {
            //             if case.protocol.protocol_id() == SupportProtocols::Relay.protocol_id() {
            //                 "relay"
            //             } else {
            //                 "relay_v2"
            //             }
            //         },
            //         {
            //             match case.tx_script_hash_type {
            //                 ScriptHashType::Data => "data",
            //                 ScriptHashType::Type => "type",
            //                 ScriptHashType::Data1 => "data1",
            //             }
            //         },
            //         {
            //             match case.relayed_cycles {
            //                 ScriptHashType::Data => "vm0-cycles",
            //                 ScriptHashType::Data1 => "vm1-cycles",
            //                 ScriptHashType::Type => "",
            //             }
            //         },
            //         actual_result,
            //     );

            //     ckb_testkit::info!("case_params {:?}", {
            //         let mut case2 = case.clone();
            //         case2.expected_result = actual_result.clone();
            //         case2
            //     });
            // }

            assert_eq!(
                case.expected_result,
                actual_result,
                "case.id: {}, expected: {:?}, actual: {:?}, node.log_path: {}, tx.hash: {:#x}",
                case.id,
                case.expected_result,
                actual_result,
                node.log_path().to_string_lossy(),
                tx.hash(),
            );
        }
    }
}

impl RFC0035RelayTransaction {
    // Start a fresh node according to configurations spefified by `case`, and
    // synchronize its chain from `base_chain_node`
    fn setup_node(&self, case: &CaseParams, base_chain_node: &Node) -> Node {
        // We only test v0.100
        let is_ckb2021 = true;
        let node_options = NodeOptions {
            node_name: format!("case-{}", case.id),
            ckb_binary: {
                if is_ckb2021 {
                    CKB2021.read().unwrap().clone()
                } else {
                    CKB2019.read().unwrap().clone()
                }
            },
            chain_spec: {
                if is_ckb2021 {
                    "testdata/spec/ckb2021"
                } else {
                    "testdata/spec/ckb2019"
                }
            },
            app_config: {
                if is_ckb2021 {
                    "testdata/config/ckb2021"
                } else {
                    "testdata/config/ckb2019"
                }
            },
            initial_database: "testdata/db/empty",
        };
        let node = {
            let mut node = Node::init(self.case_name(), node_options, is_ckb2021);
            node.start();
            node
        };

        node.pull_node(base_chain_node).unwrap();
        node.mine_to(case.node_tip);
        // TODO FIXME node may need time to switch to fork2021
        ::std::thread::sleep(Duration::from_secs(2));
        node
    }

    // Start a connector
    fn setup_connector(&self, case: &CaseParams, consensus: &Consensus) -> Connector {
        // `Sync` protocol is required by CKB `ProtocolTypeCheckerService`
        let protocols = vec![case.protocol.clone(), SupportProtocols::Sync];
        let version = &case.peer_version;
        Connector::start(
            &format!("{}-{}", self.case_name(), case.id),
            consensus,
            version,
            protocols,
        )
    }

    // Run case.
    //
    // 1. Connect the target node via p2p connection
    // 2. Setup specified protocol
    // 3. Relay specified transaction
    // 4. Wait the result
    fn run(
        &self,
        case: &CaseParams,
        connector: &mut Connector,
        node: &Node,
        transaction: &TransactionView,
        relayed_cycles: Cycle,
    ) -> Result<(), Error> {
        let _peer_index = connector
            .connect(&node)
            .map_err(|_| Error::ConnectionTimeout)?;

        connector
            .send_relay_transaction_hash(&case.protocol, &node, &transaction)
            .unwrap();
        let received_get_relay_txs = wait_until(20, || {
            if let Ok((protocol_id, data)) =
                connector.receive_timeout(node, Duration::from_secs(20))
            {
                if protocol_id == case.protocol.protocol_id() {
                    return RelayMessageReader::from_compatible_slice(&data)
                        .unwrap()
                        .to_enum()
                        .item_name()
                        == "GetRelayTransactions";
                }
            }
            false
        });
        if !received_get_relay_txs {
            return Err(Error::RelayTransactionHashFailed);
        }

        connector
            .send_relay_transaction(&case.protocol, &node, &transaction, relayed_cycles)
            .unwrap();

        let tx_relayed = wait_until(5, || node.is_transaction_pending(transaction));
        let banned = wait_until(5, || {
            let banned_addresses = node.rpc_client().get_banned_addresses();
            !banned_addresses.is_empty()
        });

        match (tx_relayed, banned) {
            (true, false) => Ok(()),
            _ => Err(Error::RelayTransactionFailed),
        }
    }

    fn build_transaction(
        node: &Node,
        input: &CellMeta,
        type_script_hash_type: ScriptHashType,
    ) -> TransactionView {
        assert!(input.cell_output.lock().hash_type() == ScriptHashType::Data.into());

        let type_ = Self::build_always_success_script(node, type_script_hash_type);
        let output = CellOutput::new_builder()
            .lock(input.cell_output.lock())
            .type_(Some(type_).pack())
            .capacity(input.capacity().pack())
            .build();
        TransactionBuilder::default()
            .input(CellInput::new(input.out_point.clone(), 0))
            .output(output)
            .output_data(Default::default())
            .cell_dep(node.always_success_cell_dep())
            .build()
    }

    fn build_always_success_script(node: &Node, script_hash_type: ScriptHashType) -> Script {
        let always_script_data_hash = {
            let genesis_cellbase_hash = node.genesis_cellbase_hash();
            let always_success_out_point =
                OutPoint::new(genesis_cellbase_hash, SYSTEM_CELL_ALWAYS_SUCCESS_INDEX);
            let cell = node
                .rpc_client()
                .get_live_cell(always_success_out_point.into(), true);
            let cell_info = cell.cell.expect("genesis always cell must be live");
            let cell_data_hash = cell_info.data.unwrap().hash;
            cell_data_hash.pack()
        };
        let always_script_type_hash = {
            let script = node.always_success_script();
            assert!(script.hash_type() == ScriptHashType::Type.into());
            script.code_hash()
        };
        match script_hash_type {
            ScriptHashType::Data => Script::new_builder()
                .code_hash(always_script_data_hash)
                .hash_type(ScriptHashType::Data.into())
                .build(),
            ScriptHashType::Type => Script::new_builder()
                .code_hash(always_script_type_hash)
                .hash_type(ScriptHashType::Type.into())
                .build(),
            ScriptHashType::Data1 => Script::new_builder()
                .code_hash(always_script_data_hash)
                .hash_type(ScriptHashType::Data1.into())
                .build(),
        }
    }

    fn filter_cases_params(fork_switch_height: BlockNumber) -> Vec<CaseParams> {
        let cases = if let Some(c_str) = ::std::env::var_os("DEBUG_CASE_ID") {
            match c_str.to_string_lossy().parse::<usize>() {
                Ok(debug_case_id) => Self::cases_params(fork_switch_height)
                    .into_iter()
                    .filter(|c| c.id == debug_case_id)
                    .collect::<Vec<_>>(),
                Err(_) => Self::cases_params(fork_switch_height),
            }
        } else {
            Self::cases_params(fork_switch_height)
        };
        assert!(!cases.is_empty());
        cases
    }

    fn cases_params(fork_switch_height: BlockNumber) -> Vec<CaseParams> {
        vec![
            CaseParams {
                id: 1,
                node_tip: fork_switch_height - 2,
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 2,
                node_tip: fork_switch_height - 2,
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 3,
                node_tip: fork_switch_height - 2,
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 4,
                node_tip: fork_switch_height - 2,
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 5,
                node_tip: fork_switch_height - 2,
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 6,
                node_tip: fork_switch_height - 2,
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 7,
                node_tip: fork_switch_height - 2,
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::RelayTransactionHashFailed),
            },
            CaseParams {
                id: 8,
                node_tip: fork_switch_height - 2,
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 9,
                node_tip: fork_switch_height - 2,
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 10,
                node_tip: fork_switch_height - 2,
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 11,
                node_tip: fork_switch_height - 2,
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 12,
                node_tip: fork_switch_height - 2,
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 13,
                node_tip: fork_switch_height - 2,
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 14,
                node_tip: fork_switch_height - 2,
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::RelayTransactionHashFailed),
            },
            CaseParams {
                id: 15,
                node_tip: fork_switch_height - 1,
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 16,
                node_tip: fork_switch_height - 1,
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 17,
                node_tip: fork_switch_height - 1,
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 18,
                node_tip: fork_switch_height - 1,
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 19,
                node_tip: fork_switch_height - 1,
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 20,
                node_tip: fork_switch_height - 1,
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 21,
                node_tip: fork_switch_height - 1,
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::RelayTransactionHashFailed),
            },
            CaseParams {
                id: 22,
                node_tip: fork_switch_height - 1,
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 23,
                node_tip: fork_switch_height - 1,
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 24,
                node_tip: fork_switch_height - 1,
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 25,
                node_tip: fork_switch_height - 1,
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 26,
                node_tip: fork_switch_height - 1,
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::RelayTransactionFailed),
            },
            CaseParams {
                id: 27,
                node_tip: fork_switch_height - 1,
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
        ]
    }
}

#[derive(Clone, Debug)]
struct CaseParams {
    id: usize,

    // The target node's tip number.
    node_tip: BlockNumber,

    // The peer's client version, CKB2019 or CKB2021.
    peer_version: String,

    // The network protocol of sending `RelayTransactions` through, Relay or
    // RelayV2.
    protocol: SupportProtocols,

    // The transaction's type-script's hash-type, indicates the version specified
    // VM that transaction runs on, `ScriptHashType::Data`,
    // `ScriptHashType::Type`, `ScriptHashType::Data1`.
    tx_script_hash_type: ScriptHashType,

    // Transaction cycles attached on `RelayTransaction` message,
    // `ScriptHashType::Data`, `ScriptHashType::Data1`.
    relayed_cycles: ScriptHashType,

    // Expected result.
    expected_result: Result<(), Error>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Error {
    ConnectionTimeout,
    RelayTransactionHashFailed,
    RelayTransactionFailed,
}
