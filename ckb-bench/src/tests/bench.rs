use crate::prepare::derive_privkeys;
use crate::tests::node_options;
use crate::{clap_app, entrypoint};
use ckb_testkit::ckb_types::packed::Byte32;
use ckb_testkit::ckb_types::prelude::*;
use ckb_testkit::ckb_types::H256;
use ckb_testkit::{Node, Nodes, User};
use std::env;
use std::str::FromStr;
use std::thread::spawn;

#[test]
fn test_bench() {
    let n_users = 1000usize;
    let cells_per_user = 2;
    let capacity_per_cell = 7100000000u64;
    let n_inout = 2usize;
    let owner_raw_privkey = "8c296482b9b763e8be974058272f377462f2975b94454dabb112de0f135e2064";
    env::set_var("CKB_BENCH_OWNER_PRIVKEY", owner_raw_privkey);

    let nodes: Nodes = node_options()
        .into_iter()
        .map(|node_options| {
            let mut node = Node::init("test_bench", node_options, true);
            node.start();
            node
        })
        .collect::<Vec<_>>()
        .into();
    let raw_nodes_urls = nodes
        .nodes()
        .map(|node| node.rpc_client().url())
        .collect::<Vec<_>>()
        .join(",");
    let node = nodes.get_node("node2021_1");
    let users: Vec<_> = {
        let genesis_block = node.get_block_by_number(0);
        let owner_byte32_privkey = {
            let h256 = H256::from_str(owner_raw_privkey).unwrap();
            Byte32::from_slice(h256.as_bytes()).unwrap()
        };
        derive_privkeys(owner_byte32_privkey, n_users)
            .into_iter()
            .map(|privkey| User::new(genesis_block.clone(), Some(privkey)))
            .collect()
    };
    assert_eq!(users.len(), n_users);

    {
        // Mine some CKBs
        node.mine(100);
        nodes.p2p_connect();
        nodes.waiting_for_sync().expect("nodes should be synced");
    }

    // Spawn mining blocks at background
    let _miner_guard = {
        let matches = clap_app().get_matches_from(vec![
            "./target/debug/ckb-bench",
            "miner",
            "--n-blocks",
            "0",
            "--mining-interval-ms",
            "1000",
            "--rpc-urls",
            &raw_nodes_urls,
        ]);
        spawn(move || {
            entrypoint(matches);
        })
    };

    // Dispatch capacity to users
    {
        for user in users.iter() {
            let cells = user.get_spendable_single_secp256k1_cells(node);
            assert!(cells.is_empty());
        }
        entrypoint(clap_app().get_matches_from(vec![
            "./target/debug/ckb-bench",
            "dispatch",
            "--data-dir",
            &node.working_dir().display().to_string(),
            "--n-users",
            n_users.to_string().as_str(),
            "--cells-per-user",
            cells_per_user.to_string().as_str(),
            "--capacity-per-cell",
            capacity_per_cell.to_string().as_str(),
            "--rpc-urls",
            &raw_nodes_urls,
        ]));
        loop {
            node.mine(1);
            let tx_pool_info = node.rpc_client().tx_pool_info();
            if tx_pool_info.pending.value() == 0
                && tx_pool_info.proposed.value() == 0
                && tx_pool_info.orphan.value() == 0
            {
                break;
            }
        }
        nodes.p2p_connect();
        nodes.waiting_for_sync().expect("nodes should be synced");

        for (i, user) in users.iter().enumerate() {
            let cells = user.get_spendable_single_secp256k1_cells(node);
            assert_eq!(
                cells.len(),
                cells_per_user,
                "user-{}'s cells actual: {}, expected: {}",
                i,
                cells.len(),
                cells_per_user
            );
            assert!(
                cells
                    .iter()
                    .all(|cell| cell.capacity().as_u64() == capacity_per_cell),
                "user-{}'s cell actual capacity, expected: {}",
                i,
                capacity_per_cell
            );
        }
    }

    // Bench
    {
        entrypoint(clap_app().get_matches_from(vec![
            "./target/debug/ckb-bench",
            "bench",
            "--data-dir",
            &node.working_dir().display().to_string(),
            "--n-users",
            n_users.to_string().as_str(),
            "--n-inout",
            n_inout.to_string().as_str(),
            "--tx-interval-ms",
            1.to_string().as_str(),
            "--bench-time-ms",
            5000.to_string().as_str(),
            "--rpc-urls",
            &raw_nodes_urls,
        ]));
    }
}
