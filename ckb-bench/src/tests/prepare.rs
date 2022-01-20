use crate::prepare::derive_privkeys;
use crate::tests::node_options;
use crate::{clap_app, entrypoint, init_logger};
use ckb_testkit::ckb_types::packed::Byte32;
use ckb_testkit::ckb_types::prelude::*;
use ckb_testkit::ckb_types::H256;
use ckb_testkit::util::wait_until;
use ckb_testkit::{Node, Nodes, User};
use std::env;
use std::str::FromStr;
use std::time::{Duration, Instant};

#[test]
fn test_prepare() {
    let _logger = init_logger();
    let owner_raw_privkey = "8c296482b9b763e8be974058272f377462f2975b94454dabb112de0f135e2064";
    env::set_var("CKB_BENCH_OWNER_PRIVKEY", owner_raw_privkey);

    let nodes: Nodes = node_options()
        .into_iter()
        .map(|node_options| {
            let mut node = Node::init("test_prepare", node_options, true);
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
    let n_users = 10000;
    let cells_per_user = 11;
    let capacity_per_cell = 7100000000u64;
    let genesis_block = node.get_block_by_number(0);
    let owner_byte32_privkey = {
        let h256 = H256::from_str(owner_raw_privkey).unwrap();
        Byte32::from_slice(h256.as_bytes()).unwrap()
    };
    let users: Vec<_> = derive_privkeys(owner_byte32_privkey, n_users)
        .into_iter()
        .map(|privkey| User::new(genesis_block.clone(), Some(privkey)))
        .collect();
    assert_eq!(users.len(), n_users);

    {
        // Mine some blocks
        const INITIAL_PRIMARY_EPOCH_REWARD: u64 = 1_917_808_21917808;
        const MAX_EPOCH_LENGTH: u64 = 1800;
        const FEE_RATE_OF_OUTPUT: u64 = 1000;

        let total_capacity = n_users as u64 * cells_per_user as u64 * capacity_per_cell;
        let total_fee = n_users as u64 * cells_per_user * FEE_RATE_OF_OUTPUT;
        let to_mine_blocks =
            (total_capacity + total_fee) / (INITIAL_PRIMARY_EPOCH_REWARD / MAX_EPOCH_LENGTH) + 100;
        println!(
            "total_capacity({}) = n_users({}) * cells_per_user({}) * capacity_per_cell({})",
            total_capacity, n_users, cells_per_user, capacity_per_cell,
        );
        println!(
            "total_fee({}) = n_users({}) * cells_per_user({}) * FEE_RATE_OF_OUTPUT({})",
            total_fee, n_users, cells_per_user, FEE_RATE_OF_OUTPUT,
        );
        println!(
            "to_mine_blocks({}) = (total_capacity({}) + total_fee({})) / (INITIAL_PRIMARY_EPOCH_REWARD({}) / MAX_EPOCH_LENGTH({})) + 100",
            to_mine_blocks, total_capacity, total_fee, INITIAL_PRIMARY_EPOCH_REWARD, MAX_EPOCH_LENGTH
        );
        node.mine(to_mine_blocks);
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
            "100",
            "--rpc-urls",
            &raw_nodes_urls,
        ]);
        ::std::thread::spawn(move || {
            entrypoint(matches);
        })
    };

    {
        // Dispatch capacity to users
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
        for _ in 0..10 {
            for node in nodes.nodes() {
                node.mine(1);
                nodes.p2p_connect();
                nodes.waiting_for_sync().expect("nodes should be synced");
            }
        }
        for (i, user) in users.iter().enumerate() {
            let cells = user.get_spendable_single_secp256k1_cells(node);
            let total_capacity: u64 = cells.iter().map(|cell| cell.capacity().as_u64()).sum();
            assert_eq!(
                total_capacity,
                cells_per_user * capacity_per_cell,
                "user-{} actual capacity: {}, expected capacity: {}",
                i,
                total_capacity,
                cells_per_user * capacity_per_cell,
            );
        }
    }

    {
        // Collect users' cells to owner
        entrypoint(clap_app().get_matches_from(vec![
            "./target/debug/ckb-bench",
            "collect",
            "--data-dir",
            &node.working_dir().display().to_string(),
            "--n-users",
            n_users.to_string().as_str(),
            "--rpc-urls",
            &raw_nodes_urls,
        ]));

        let ret = wait_until(600, || {
            for node in nodes.nodes() {
                let tx_pool_info = node.get_tip_tx_pool_info();
                if tx_pool_info.proposed.value() != 0
                    || tx_pool_info.pending.value() != 0
                    || tx_pool_info.orphan.value() != 0
                    || tx_pool_info.total_tx_size.value() != 0
                    || tx_pool_info.total_tx_cycles.value() != 0
                {
                    node.mine(1);
                    nodes.p2p_connect();
                    nodes.waiting_for_sync().expect("nodes should be synced");

                    return false;
                }
            }

            true
        });
        assert!(ret, "timeout to reset the tx-pool");

        for user in users {
            let cells = user.get_spendable_single_secp256k1_cells(node);
            let total_capacity: u64 = cells.iter().map(|cell| cell.capacity().as_u64()).sum();
            assert_eq!(total_capacity, 0);
        }
    }
}
