use crate::prepare::generate_privkeys;
use crate::tests::node_options;
use crate::{clap_app, entrypoint};
use ckb_testkit::{Node, Nodes, User};
use ckb_types::packed::Byte32;
use ckb_types::prelude::*;
use ckb_types::H256;
use std::env;
use std::str::FromStr;

#[test]
fn test_prepare() {
    let lender_raw_privkey = "8c296482b9b763e8be974058272f377462f2975b94454dabb112de0f135e2064";
    env::set_var("CKB_BENCH_LENDER_PRIVKEY", lender_raw_privkey);

    let nodes: Nodes = node_options()
        .into_iter()
        .map(|node_options| {
            let mut node = Node::init("test_prepare", node_options, true);
            println!(
                "[Node {}] START log_path: \"{}\"",
                node.node_name(),
                node.log_path().display()
            );
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
    let n_borrowers = 10;
    let borrow_capacity = 7100000000;
    let genesis_block = node.get_block_by_number(0);
    let lender_byte32_privkey = {
        let h256 = H256::from_str(lender_raw_privkey).unwrap();
        Byte32::from_slice(h256.as_bytes()).unwrap()
    };
    let users: Vec<_> = generate_privkeys(lender_byte32_privkey, n_borrowers)
        .into_iter()
        .map(|privkey| User::new(genesis_block.clone(), Some(privkey)))
        .collect();
    assert_eq!(users.len(), n_borrowers);

    {
        // Mine some blocks
        node.mine(20);
        nodes.p2p_connect();
        nodes.waiting_for_sync().expect("nodes should be synced");
    }

    {
        // Dispatch capacity to borrowers
        for user in users.iter() {
            let cells = user.get_spendable_single_secp256k1_cells(node);
            assert!(cells.is_empty());
        }
        entrypoint(clap_app().get_matches_from(vec![
            "./target/debug/ckb-bench",
            "dispatch",
            "--working_dir",
            &node.working_dir().display().to_string(),
            "--n_borrowers",
            n_borrowers.to_string().as_str(),
            "--borrow_capacity",
            borrow_capacity.to_string().as_str(),
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
                total_capacity, borrow_capacity,
                "user-{} actual capacity: {}, expected capacity: {}",
                i, total_capacity, borrow_capacity,
            );
        }
    }

    {
        // Collect borrowers' cells to lender
        entrypoint(clap_app().get_matches_from(vec![
            "./target/debug/ckb-bench",
            "collect",
            "--working_dir",
            &node.working_dir().display().to_string(),
            "--n_borrowers",
            n_borrowers.to_string().as_str(),
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
                total_capacity, 0,
                "user-{} actual capacity: {}, expected capacity: {}",
                i, total_capacity, 0,
            );
        }
    }
}
