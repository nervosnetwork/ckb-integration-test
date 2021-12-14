use crate::tests::node_options;
use crate::{clap_app, entrypoint, init_logger};
use ckb_testkit::{Node, Nodes};
use std::env;

#[test]
fn test_mine() {
    let _logger = init_logger();
    let owner_raw_privkey = "8c296482b9b763e8be974058272f377462f2975b94454dabb112de0f135e2064";
    env::set_var("CKB_BENCH_OWNER_PRIVKEY", owner_raw_privkey);

    let nodes: Nodes = node_options()
        .into_iter()
        .map(|node_options| {
            let mut node = Node::init("test_mine", node_options, true);
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
    let old_tip_number = nodes.get_fixed_header().number();

    {
        // Mine some blocks
        let matches = clap_app().get_matches_from(vec![
            "./target/debug/ckb-bench",
            "miner",
            "--n-blocks",
            "100",
            "--mining-interval-ms",
            "1",
            "--rpc-urls",
            &raw_nodes_urls,
        ]);
        entrypoint(matches);
    }

    {
        nodes.p2p_connect();
        nodes.waiting_for_sync().expect("nodes should be synced");
        let new_tip_number = nodes.get_fixed_header().number();
        assert_ne!(old_tip_number, new_tip_number);
    }
}
