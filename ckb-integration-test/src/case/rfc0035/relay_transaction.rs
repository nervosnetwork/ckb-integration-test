use super::RFC0035_EPOCH_NUMBER;
use crate::{
    preclude::*,
    util::{calc_epoch_start_number, v0_100, v0_43, Connector},
};
use ckb_jsonrpc_types::Consensus;
use ckb_network::SupportProtocols;
use ckb_testkit::util::wait_until;
use ckb_testkit::SYSTEM_CELL_ALWAYS_SUCCESS_INDEX;
use ckb_types::{
    core::{
        cell::CellMeta, BlockNumber, Cycle, ScriptHashType, TransactionBuilder, TransactionView,
    },
    packed::{CellInput, CellOutput, OutPoint, Script},
    prelude::*,
};

struct CaseParams {
    id: usize,

    // The target node's tip number.
    //
    // We use this parameter to control the fork2021 activation.
    // When `node_tip < fork_switch_height - 3`, the test node does not activate
    // fork2021; vice verse.
    node_tip: BlockNumber,

    // The target node's client version, CKB2019 or CKB2021.
    node_version: String,

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
    TransactionRelayedAndPeerBanned,
    TransactionNotRelayedAndPeerBanned,
    TransactionNotRelayedAndPeerNotBanned,
}

/// Cases
///
/// ```text
/// ┌─────┬──────┬─────────┬─────────┬──────────┬───────────┬──────────────┬────────────────────────────────────────────
/// │ id  │ node │ node    │ peer    │ protocol │ tx.script │ relayed      │ result                                     │
/// │     │ tip  │ version │ version │          │ hash_type │ cycles       │                                            │
/// └─────┴──────┴─────────┴─────────┴──────────┴───────────┴──────────────┴────────────────────────────────────────────
/// │ 0   │ 2996 │ v0.43   │ v0.43   │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 1   │ 2996 │ v0.43   │ v0.43   │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 2   │ 2996 │ v0.43   │ v0.43   │ relay    │ type      │ data0-cycles │ Ok(())                                     │
/// │ 3   │ 2996 │ v0.43   │ v0.43   │ relay    │ type      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 4   │ 2996 │ v0.43   │ v0.43   │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 5   │ 2996 │ v0.43   │ v0.43   │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 6   │ 2996 │ v0.43   │ v0.43   │ relay_v2 │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 7   │ 2996 │ v0.43   │ v0.43   │ relay_v2 │ data      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 8   │ 2996 │ v0.43   │ v0.43   │ relay_v2 │ type      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 9   │ 2996 │ v0.43   │ v0.43   │ relay_v2 │ type      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 10  │ 2996 │ v0.43   │ v0.43   │ relay_v2 │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 11  │ 2996 │ v0.43   │ v0.43   │ relay_v2 │ data1     │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 12  │ 2996 │ v0.43   │ v0.100  │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 13  │ 2996 │ v0.43   │ v0.100  │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 14  │ 2996 │ v0.43   │ v0.100  │ relay    │ type      │ data0-cycles │ Ok(())                                     │
/// │ 15  │ 2996 │ v0.43   │ v0.100  │ relay    │ type      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 16  │ 2996 │ v0.43   │ v0.100  │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 17  │ 2996 │ v0.43   │ v0.100  │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 18  │ 2996 │ v0.43   │ v0.100  │ relay_v2 │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 19  │ 2996 │ v0.43   │ v0.100  │ relay_v2 │ data      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 20  │ 2996 │ v0.43   │ v0.100  │ relay_v2 │ type      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 21  │ 2996 │ v0.43   │ v0.100  │ relay_v2 │ type      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 22  │ 2996 │ v0.43   │ v0.100  │ relay_v2 │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 23  │ 2996 │ v0.43   │ v0.100  │ relay_v2 │ data1     │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 24  │ 2996 │ v0.100  │ v0.43   │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 25  │ 2996 │ v0.100  │ v0.43   │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 26  │ 2996 │ v0.100  │ v0.43   │ relay    │ type      │ data0-cycles │ Ok(())                                     │
/// │ 27  │ 2996 │ v0.100  │ v0.43   │ relay    │ type      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 28  │ 2996 │ v0.100  │ v0.43   │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 29  │ 2996 │ v0.100  │ v0.43   │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 30  │ 2996 │ v0.100  │ v0.43   │ relay_v2 │ data      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 31  │ 2996 │ v0.100  │ v0.43   │ relay_v2 │ data      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 32  │ 2996 │ v0.100  │ v0.43   │ relay_v2 │ type      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 33  │ 2996 │ v0.100  │ v0.43   │ relay_v2 │ type      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 34  │ 2996 │ v0.100  │ v0.43   │ relay_v2 │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 35  │ 2996 │ v0.100  │ v0.43   │ relay_v2 │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 36  │ 2996 │ v0.100  │ v0.100  │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 37  │ 2996 │ v0.100  │ v0.100  │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 38  │ 2996 │ v0.100  │ v0.100  │ relay    │ type      │ data0-cycles │ Ok(())                                     │
/// │ 39  │ 2996 │ v0.100  │ v0.100  │ relay    │ type      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 40  │ 2996 │ v0.100  │ v0.100  │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 41  │ 2996 │ v0.100  │ v0.100  │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 42  │ 2996 │ v0.100  │ v0.100  │ relay_v2 │ data      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 43  │ 2996 │ v0.100  │ v0.100  │ relay_v2 │ data      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 44  │ 2996 │ v0.100  │ v0.100  │ relay_v2 │ type      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 45  │ 2996 │ v0.100  │ v0.100  │ relay_v2 │ type      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 46  │ 2996 │ v0.100  │ v0.100  │ relay_v2 │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 47  │ 2996 │ v0.100  │ v0.100  │ relay_v2 │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 48  │ 2997 │ v0.43   │ v0.43   │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 49  │ 2997 │ v0.43   │ v0.43   │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 50  │ 2997 │ v0.43   │ v0.43   │ relay    │ type      │ data0-cycles │ Ok(())                                     │
/// │ 51  │ 2997 │ v0.43   │ v0.43   │ relay    │ type      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 52  │ 2997 │ v0.43   │ v0.43   │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 53  │ 2997 │ v0.43   │ v0.43   │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 54  │ 2997 │ v0.43   │ v0.43   │ relay_v2 │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 55  │ 2997 │ v0.43   │ v0.43   │ relay_v2 │ data      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 56  │ 2997 │ v0.43   │ v0.43   │ relay_v2 │ type      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 57  │ 2997 │ v0.43   │ v0.43   │ relay_v2 │ type      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 58  │ 2997 │ v0.43   │ v0.43   │ relay_v2 │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 59  │ 2997 │ v0.43   │ v0.43   │ relay_v2 │ data1     │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 60  │ 2997 │ v0.43   │ v0.100  │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 61  │ 2997 │ v0.43   │ v0.100  │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 62  │ 2997 │ v0.43   │ v0.100  │ relay    │ type      │ data0-cycles │ Ok(())                                     │
/// │ 63  │ 2997 │ v0.43   │ v0.100  │ relay    │ type      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 64  │ 2997 │ v0.43   │ v0.100  │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 65  │ 2997 │ v0.43   │ v0.100  │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 66  │ 2997 │ v0.43   │ v0.100  │ relay_v2 │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 67  │ 2997 │ v0.43   │ v0.100  │ relay_v2 │ data      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 68  │ 2997 │ v0.43   │ v0.100  │ relay_v2 │ type      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 69  │ 2997 │ v0.43   │ v0.100  │ relay_v2 │ type      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 70  │ 2997 │ v0.43   │ v0.100  │ relay_v2 │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 71  │ 2997 │ v0.43   │ v0.100  │ relay_v2 │ data1     │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 72  │ 2997 │ v0.100  │ v0.43   │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 73  │ 2997 │ v0.100  │ v0.43   │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 74? │ 2997 │ v0.100  │ v0.43   │ relay    │ type      │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 75  │ 2997 │ v0.100  │ v0.43   │ relay    │ type      │ data1-cycles │ Ok(())                                     │
/// │ 76? │ 2997 │ v0.100  │ v0.43   │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 77  │ 2997 │ v0.100  │ v0.43   │ relay    │ data1     │ data1-cycles │ Ok(())                                     │
/// │ 78? │ 2997 │ v0.100  │ v0.43   │ relay_v2 │ data      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 79? │ 2997 │ v0.100  │ v0.43   │ relay_v2 │ data      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 80? │ 2997 │ v0.100  │ v0.43   │ relay_v2 │ type      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 81? │ 2997 │ v0.100  │ v0.43   │ relay_v2 │ type      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 82? │ 2997 │ v0.100  │ v0.43   │ relay_v2 │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 83? │ 2997 │ v0.100  │ v0.43   │ relay_v2 │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 84  │ 2997 │ v0.100  │ v0.100  │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 85  │ 2997 │ v0.100  │ v0.100  │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 86? │ 2997 │ v0.100  │ v0.100  │ relay    │ type      │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 87  │ 2997 │ v0.100  │ v0.100  │ relay    │ type      │ data1-cycles │ Ok(())                                     │
/// │ 88? │ 2997 │ v0.100  │ v0.100  │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 89  │ 2997 │ v0.100  │ v0.100  │ relay    │ data1     │ data1-cycles │ Ok(())                                     │
/// │ 90? │ 2997 │ v0.100  │ v0.100  │ relay_v2 │ data      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 91? │ 2997 │ v0.100  │ v0.100  │ relay_v2 │ data      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 92? │ 2997 │ v0.100  │ v0.100  │ relay_v2 │ type      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 93? │ 2997 │ v0.100  │ v0.100  │ relay_v2 │ type      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 94? │ 2997 │ v0.100  │ v0.100  │ relay_v2 │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 95? │ 2997 │ v0.100  │ v0.100  │ relay_v2 │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 96  │ 2999 │ v0.43   │ v0.43   │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 97  │ 2999 │ v0.43   │ v0.43   │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 98  │ 2999 │ v0.43   │ v0.43   │ relay    │ type      │ data0-cycles │ Ok(())                                     │
/// │ 99  │ 2999 │ v0.43   │ v0.43   │ relay    │ type      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 100 │ 2999 │ v0.43   │ v0.43   │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 101 │ 2999 │ v0.43   │ v0.43   │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 102 │ 2999 │ v0.43   │ v0.43   │ relay_v2 │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 103 │ 2999 │ v0.43   │ v0.43   │ relay_v2 │ data      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 104 │ 2999 │ v0.43   │ v0.43   │ relay_v2 │ type      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 105 │ 2999 │ v0.43   │ v0.43   │ relay_v2 │ type      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 106 │ 2999 │ v0.43   │ v0.43   │ relay_v2 │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 107 │ 2999 │ v0.43   │ v0.43   │ relay_v2 │ data1     │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 108 │ 2999 │ v0.43   │ v0.100  │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 109 │ 2999 │ v0.43   │ v0.100  │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 110 │ 2999 │ v0.43   │ v0.100  │ relay    │ type      │ data0-cycles │ Ok(())                                     │
/// │ 111 │ 2999 │ v0.43   │ v0.100  │ relay    │ type      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 112 │ 2999 │ v0.43   │ v0.100  │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 113 │ 2999 │ v0.43   │ v0.100  │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 114 │ 2999 │ v0.43   │ v0.100  │ relay_v2 │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 115 │ 2999 │ v0.43   │ v0.100  │ relay_v2 │ data      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 116 │ 2999 │ v0.43   │ v0.100  │ relay_v2 │ type      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 117 │ 2999 │ v0.43   │ v0.100  │ relay_v2 │ type      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 118 │ 2999 │ v0.43   │ v0.100  │ relay_v2 │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 119 │ 2999 │ v0.43   │ v0.100  │ relay_v2 │ data1     │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 120 │ 2999 │ v0.100  │ v0.43   │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 121 │ 2999 │ v0.100  │ v0.43   │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 122 │ 2999 │ v0.100  │ v0.43   │ relay    │ type      │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 123 │ 2999 │ v0.100  │ v0.43   │ relay    │ type      │ data1-cycles │ Ok(())                                     │
/// │ 124 │ 2999 │ v0.100  │ v0.43   │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 125 │ 2999 │ v0.100  │ v0.43   │ relay    │ data1     │ data1-cycles │ Ok(())                                     │
/// │ 126 │ 2999 │ v0.100  │ v0.43   │ relay_v2 │ data      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 127 │ 2999 │ v0.100  │ v0.43   │ relay_v2 │ data      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 128 │ 2999 │ v0.100  │ v0.43   │ relay_v2 │ type      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 129 │ 2999 │ v0.100  │ v0.43   │ relay_v2 │ type      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 130 │ 2999 │ v0.100  │ v0.43   │ relay_v2 │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 131 │ 2999 │ v0.100  │ v0.43   │ relay_v2 │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 132 │ 2999 │ v0.100  │ v0.100  │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 133?│ 2999 │ v0.100  │ v0.100  │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 134?│ 2999 │ v0.100  │ v0.100  │ relay    │ type      │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 135 │ 2999 │ v0.100  │ v0.100  │ relay    │ type      │ data1-cycles │ Ok(())                                     │
/// │ 136 │ 2999 │ v0.100  │ v0.100  │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 137 │ 2999 │ v0.100  │ v0.100  │ relay    │ data1     │ data1-cycles │ Ok(())                                     │
/// │ 138 │ 2999 │ v0.100  │ v0.100  │ relay_v2 │ data      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 139 │ 2999 │ v0.100  │ v0.100  │ relay_v2 │ data      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 140 │ 2999 │ v0.100  │ v0.100  │ relay_v2 │ type      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 141 │ 2999 │ v0.100  │ v0.100  │ relay_v2 │ type      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 142 │ 2999 │ v0.100  │ v0.100  │ relay_v2 │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 143 │ 2999 │ v0.100  │ v0.100  │ relay_v2 │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 144 │ 3000 │ v0.43   │ v0.43   │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 145 │ 3000 │ v0.43   │ v0.43   │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 146 │ 3000 │ v0.43   │ v0.43   │ relay    │ type      │ data0-cycles │ Ok(())                                     │
/// │ 147 │ 3000 │ v0.43   │ v0.43   │ relay    │ type      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 148 │ 3000 │ v0.43   │ v0.43   │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 149 │ 3000 │ v0.43   │ v0.43   │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 150 │ 3000 │ v0.43   │ v0.43   │ relay_v2 │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 151 │ 3000 │ v0.43   │ v0.43   │ relay_v2 │ data      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 152 │ 3000 │ v0.43   │ v0.43   │ relay_v2 │ type      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 153 │ 3000 │ v0.43   │ v0.43   │ relay_v2 │ type      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 154 │ 3000 │ v0.43   │ v0.43   │ relay_v2 │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 155 │ 3000 │ v0.43   │ v0.43   │ relay_v2 │ data1     │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 156 │ 3000 │ v0.43   │ v0.100  │ relay    │ data      │ data0-cycles │ Ok(())                                     │
/// │ 157 │ 3000 │ v0.43   │ v0.100  │ relay    │ data      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 158 │ 3000 │ v0.43   │ v0.100  │ relay    │ type      │ data0-cycles │ Ok(())                                     │
/// │ 159 │ 3000 │ v0.43   │ v0.100  │ relay    │ type      │ data1-cycles │ Err(TransactionRelayedAndPeerBanned)       │
/// │ 160 │ 3000 │ v0.43   │ v0.100  │ relay    │ data1     │ data0-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 161 │ 3000 │ v0.43   │ v0.100  │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerBanned)    │
/// │ 162 │ 3000 │ v0.43   │ v0.100  │ relay_v2 │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 163 │ 3000 │ v0.43   │ v0.100  │ relay_v2 │ data      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 164 │ 3000 │ v0.43   │ v0.100  │ relay_v2 │ type      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 165 │ 3000 │ v0.43   │ v0.100  │ relay_v2 │ type      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 166 │ 3000 │ v0.43   │ v0.100  │ relay_v2 │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 167 │ 3000 │ v0.43   │ v0.100  │ relay_v2 │ data1     │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 168 │ 3000 │ v0.100  │ v0.43   │ relay    │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 169 │ 3000 │ v0.100  │ v0.43   │ relay    │ data      │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 170 │ 3000 │ v0.100  │ v0.43   │ relay    │ type      │ data0-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 171 │ 3000 │ v0.100  │ v0.43   │ relay    │ type      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 172 │ 3000 │ v0.100  │ v0.43   │ relay    │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 173 │ 3000 │ v0.100  │ v0.43   │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 174 │ 3000 │ v0.100  │ v0.43   │ relay_v2 │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 175 │ 3000 │ v0.100  │ v0.43   │ relay_v2 │ data      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 176 │ 3000 │ v0.100  │ v0.43   │ relay_v2 │ type      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 177 │ 3000 │ v0.100  │ v0.43   │ relay_v2 │ type      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 178 │ 3000 │ v0.100  │ v0.43   │ relay_v2 │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 179 │ 3000 │ v0.100  │ v0.43   │ relay_v2 │ data1     │ data1-cycles │ Ok(())                                     │
/// │ 180 │ 3000 │ v0.100  │ v0.100  │ relay    │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 181 │ 3000 │ v0.100  │ v0.100  │ relay    │ data      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 182 │ 3000 │ v0.100  │ v0.100  │ relay    │ type      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 183 │ 3000 │ v0.100  │ v0.100  │ relay    │ type      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 184 │ 3000 │ v0.100  │ v0.100  │ relay    │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 185 │ 3000 │ v0.100  │ v0.100  │ relay    │ data1     │ data1-cycles │ Err(TransactionNotRelayedAndPeerNotBanned) │
/// │ 186 │ 3000 │ v0.100  │ v0.100  │ relay_v2 │ data      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 187 │ 3000 │ v0.100  │ v0.100  │ relay_v2 │ data      │ data1-cycles │ Err(ConnectionTimeout)                     │
/// │ 188 │ 3000 │ v0.100  │ v0.100  │ relay_v2 │ type      │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 189 │ 3000 │ v0.100  │ v0.100  │ relay_v2 │ type      │ data1-cycles │ Ok(())                                     │
/// │ 190 │ 3000 │ v0.100  │ v0.100  │ relay_v2 │ data1     │ data0-cycles │ Err(ConnectionTimeout)                     │
/// │ 191 │ 3000 │ v0.100  │ v0.100  │ relay_v2 │ data1     │ data1-cycles │ Err(ConnectionTimeout)                     │
/// └─────┴──────┴─────────┴─────────┴──────────┴───────────┴──────────────┴────────────────────────────────────────────
/// ```

// TODO 在 PR 里建议 case-85 这种 hash-type 非 type 的，如果 cycles 不一致，就以不同的方式处理？
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
                // Note: `input.script.hash_type == "data"` ensures input's script
                // consumes consistent cycles.
                app_config: "testdata/config/ckb2021_block_assembler_hash_type_is_data",
            }],
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2021 = nodes.get_node("node2021");
        node2021.mine(13);

        let fork_switch_height = calc_epoch_start_number(node2021, RFC0035_EPOCH_NUMBER);

        // Setup
        //
        // Prepare 3 kinds of transactions:
        //   - data0_tx, data0_tx.type_.hash_type = ScriptHashType::Data
        //   - type_tx,   type_tx.type_.hash_type = ScriptHashType::Type
        //   - data1_tx, data1_tx.type_.hash_type = ScriptHashType::Data1
        //
        // Calculate transaction cycles consumed when runs on VM0 and VM1:
        //   - data0_cycles, transaction cycles consumed when runs on VM0
        //   - data1_cycles, transaction cycles consumed when runs on VM1
        //
        // Node versions:
        //   - v0_43, `$CKB2019 --version`
        //   - v0_100, `$CKB2021 --version`
        let data0_tx;
        let type_tx;
        let data1_tx;
        let data0_cycles;
        let data1_cycles;
        {
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
            assert!(input.cell_output.lock().hash_type() == ScriptHashType::Data.into());
            assert!(
                input.transaction_info.as_ref().unwrap().block_number < fork_switch_height - 3,
                "Ensure that the transaction consumes this input is valid"
            );

            let node_used_to_dry_run_txs = {
                let node = node2021.clone_node("used_to_dry_run_txs");
                // Let `node_used_to_dry_run_txs` activates fork2021, so that
                // it allows data1-transactions
                node.pull_node(node2021).unwrap();
                node.mine_to(fork_switch_height);
                node
            };

            data0_tx = Self::build_transaction(node2021, &input, ScriptHashType::Data);
            type_tx = Self::build_transaction(node2021, &input, ScriptHashType::Type);
            data1_tx = Self::build_transaction(node2021, &input, ScriptHashType::Data1);
            data0_cycles = node_used_to_dry_run_txs.get_transaction_cycles(&data0_tx);
            data1_cycles = node_used_to_dry_run_txs.get_transaction_cycles(&data1_tx);
        };

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
        for case in cases {
            let tx = match case.tx_script_hash_type {
                ScriptHashType::Data => data0_tx.clone(),
                ScriptHashType::Type => type_tx.clone(),
                ScriptHashType::Data1 => data1_tx.clone(),
            };
            let relayed_cycles = match case.relayed_cycles {
                ScriptHashType::Data => data0_cycles,
                ScriptHashType::Type => unreachable!(),
                ScriptHashType::Data1 => data1_cycles,
            };
            let node = case.setup_node(node2021);
            let mut connector = case.setup_connector(node.consensus());
            let actual_result = case.run(&mut connector, &node, &tx, relayed_cycles);
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

impl CaseParams {
    const CASE_NAME: &'static str = "RFC0035RelayTransaction";

    // Start a fresh node and synchronize chain data from `base_chain_node`.
    fn setup_node(&self, base_chain_node: &Node) -> Node {
        let is_ckb2021 = self.node_version == v0_100();
        let node_options = NodeOptions {
            node_name: format!("case-{}", self.id),
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
            let mut node = Node::init(&Self::CASE_NAME, node_options, is_ckb2021);
            node.start();
            node
        };

        node.pull_node(base_chain_node).unwrap();
        node.mine_to(self.node_tip);
        node
    }

    fn setup_connector(&self, consensus: &Consensus) -> Connector {
        let protocols = vec![self.protocol.clone()];
        let version = &self.peer_version;
        Connector::start(
            &format!("{}-{}", Self::CASE_NAME, self.id),
            consensus,
            version,
            protocols,
        )
    }

    fn run(
        &self,
        connector: &mut Connector,
        node: &Node,
        transaction: &TransactionView,
        relayed_cycles: Cycle,
    ) -> Result<(), Error> {
        let _peer_index = connector
            .connect(&node)
            .map_err(|_| Error::ConnectionTimeout)?;

        match self.protocol {
            SupportProtocols::Relay => connector
                .send_relay_transaction(&node, &transaction, relayed_cycles)
                .unwrap(),
            SupportProtocols::RelayV2 => connector
                .send_relay_v2_transaction(&node, &transaction, relayed_cycles)
                .unwrap(),
            _ => unreachable!(),
        }
        let tx_relayed = wait_until(5, || node.is_transaction_pending(transaction));
        let banned = wait_until(5, || {
            let banned_addresses = node.rpc_client().get_banned_addresses();
            !banned_addresses.is_empty()
        });

        match (tx_relayed, banned) {
            (true, false) => Ok(()),
            (true, true) => Err(Error::TransactionRelayedAndPeerBanned),
            (false, false) => Err(Error::TransactionNotRelayedAndPeerNotBanned),
            (false, true) => Err(Error::TransactionNotRelayedAndPeerBanned),
        }
    }
}

impl RFC0035RelayTransaction {
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

    fn cases_params(fork_switch_height: BlockNumber) -> Vec<CaseParams> {
        vec![
            CaseParams {
                id: 0,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 1,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 2,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 3,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 4,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 5,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 6,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 7,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 8,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 9,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 10,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 11,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 12,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 13,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 14,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 15,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 16,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 17,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 18,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 19,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 20,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 21,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 22,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 23,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 24,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 25,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 26,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 27,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 28,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 29,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 30,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 31,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 32,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 33,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 34,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 35,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 36,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 37,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 38,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 39,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 40,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 41,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 42,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 43,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 44,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 45,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 46,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 47,
                node_tip: fork_switch_height - 3 - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 48,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 49,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 50,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 51,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 52,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 53,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 54,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 55,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 56,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 57,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 58,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 59,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 60,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 61,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 62,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 63,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 64,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 65,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 66,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 67,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 68,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 69,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 70,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 71,
                node_tip: fork_switch_height - 3,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 72,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 73,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 74,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 75,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 76,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 77,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 78,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 79,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 80,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 81,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 82,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 83,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 84,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 85,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 86,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 87,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 88,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 89,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 90,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 91,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 92,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 93,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 94,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 95,
                node_tip: fork_switch_height - 3,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 96,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 97,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 98,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 99,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 100,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 101,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 102,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 103,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 104,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 105,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 106,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 107,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 108,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 109,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 110,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 111,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 112,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 113,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 114,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 115,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 116,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 117,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 118,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 119,
                node_tip: fork_switch_height - 1,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 120,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 121,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 122,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 123,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 124,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 125,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 126,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 127,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 128,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 129,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 130,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 131,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 132,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 133,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 134,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 135,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 136,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 137,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 138,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 139,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 140,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 141,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 142,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 143,
                node_tip: fork_switch_height - 1,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 144,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 145,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 146,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 147,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 148,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 149,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 150,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 151,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 152,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 153,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 154,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 155,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 156,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 157,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 158,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 159,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionRelayedAndPeerBanned),
            },
            CaseParams {
                id: 160,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 161,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerBanned),
            },
            CaseParams {
                id: 162,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 163,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 164,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 165,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 166,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 167,
                node_tip: fork_switch_height,
                node_version: v0_43(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 168,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 169,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 170,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 171,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 172,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 173,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 174,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 175,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 176,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 177,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 178,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 179,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_43(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 180,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 181,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 182,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 183,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 184,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 185,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::Relay,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::TransactionNotRelayedAndPeerNotBanned),
            },
            CaseParams {
                id: 186,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 187,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 188,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 189,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Type,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Ok(()),
            },
            CaseParams {
                id: 190,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data,
                expected_result: Err(Error::ConnectionTimeout),
            },
            CaseParams {
                id: 191,
                node_tip: fork_switch_height,
                node_version: v0_100(),
                peer_version: v0_100(),
                protocol: SupportProtocols::RelayV2,
                tx_script_hash_type: ScriptHashType::Data1,
                relayed_cycles: ScriptHashType::Data1,
                expected_result: Err(Error::ConnectionTimeout),
            },
        ]
    }
}
