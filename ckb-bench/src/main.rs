
mod logger;
mod  node;
mod nodes;
pub mod util;
mod watcher;
mod utils;
mod user;
mod stat;
mod prepare;
mod bench;
#[cfg(test)]
mod tests;
mod rpc;

use tokio::runtime::Runtime;
use crate::bench::{AddTxParam, LiveCellProducer, TransactionConsumer, TransactionProducer};
use crate::prepare::{collect, derive_privkeys, dispatch};
use crate::watcher::Watcher;
use ckb_types::core::{BlockNumber};
use clap::{value_t_or_exit, values_t_or_exit, App, Arg, ArgMatches, SubCommand};
use crossbeam_channel::{bounded};
use std::env;
use std::ops::Div;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
use ckb_types::H256;
use ckb_types::packed::{Byte32};
use ckb_types::prelude::Entity;
use url::Url;
use crate::nodes::Nodes;
use crate::user::User;
use ckb_crypto::secp::{Privkey};
use crate::node::Node;


#[macro_export]
macro_rules! prompt_and_exit {
    ($($arg:tt)*) => ({
        eprintln!($($arg)*);
        crate::error!($($arg)*);
        ::std::process::exit(1);
    })
}

fn main() {
    let _logger = init_logger();
    entrypoint(clap_app().get_matches());
}

pub fn entrypoint(clap_arg_match: ArgMatches<'static>) {
    match clap_arg_match.subcommand() {
        ("miner", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let n_blocks = value_t_or_exit!(arguments, "n-blocks", u64);
            let mining_interval_ms = value_t_or_exit!(arguments, "mining-interval-ms", u64);
            let nodes: Nodes = rpc_urls
                .iter()
                .map(|url| Node::init(url.as_str(),url.as_str() ))
                .collect::<Vec<_>>()
                .into();

            // ensure nodes be out of ibd
            let max_tip_number = nodes
                .nodes()
                .map(|node| node.rpc_client().get_tip_block_number().unwrap())
                .max()
                .unwrap();
            if max_tip_number.value() == 0 {
                for node in nodes.nodes() {
                    node.mine(1);
                    break;
                }
            }

            // connect nodes
            // nodes.p2p_connect();

            let max_tip_number = nodes
                .nodes()
                .map(|node| node.rpc_client().get_tip_block_number().unwrap())
                .max()
                .unwrap();
            while nodes
                .nodes()
                .any(|node| node.rpc_client().get_tip_block_number().unwrap() < max_tip_number)
            {
                sleep(Duration::from_secs(10));
                crate::info!("wait nodes sync");
            }

            // mine `n_blocks`
            let mut mined_n_blocks = 0;
            let mut last_print_instant = Instant::now();
            loop {
                for node in nodes.nodes() {
                    node.mine(1);
                    mined_n_blocks += 1;
                    if n_blocks != 0 && mined_n_blocks >= n_blocks {
                        return;
                    }

                    if last_print_instant.elapsed() >= Duration::from_secs(10) {
                        last_print_instant = Instant::now();
                        if n_blocks == 0 {
                            crate::info!(
                                "mined {} blocks, fixed_tip_number: {}",
                                mined_n_blocks,
                                nodes.get_fixed_header().inner.number.value()
                            );
                        } else {
                            crate::info!(
                                "mined {}/{} blocks, fixed_tip_number: {}",
                                mined_n_blocks,
                                n_blocks,
                                nodes.get_fixed_header().inner.number.value()
                            );
                        }
                    }
                    if mining_interval_ms != 0 {
                        sleep(Duration::from_millis(mining_interval_ms));
                    }
                }
            }
        }
        ("dispatch", Some(arguments)) => {
            let data_dir = value_t_or_exit!(arguments, "data-dir", PathBuf);
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    let port = url.port().unwrap();
                    let host = url.host_str().unwrap();
                    let node_data_dir = data_dir.join(&format!("{}:{}", host, port));
                    ::std::fs::create_dir_all(&node_data_dir).unwrap_or_else(|err| {
                        panic!(
                            "failed to create dir \"{}\", error: {}",
                            node_data_dir.display(),
                            err
                        )
                    });

                    Node::init(url.as_str(), url.as_str())
                })
                .collect::<Vec<_>>();
            let n_users = value_t_or_exit!(arguments, "n-users", usize);
            let cells_per_user = value_t_or_exit!(arguments, "cells-per-user", u64);
            let capacity_per_cell = value_t_or_exit!(arguments, "capacity-per-cell", u64);
            let owner_raw_privkey = env::var("CKB_BENCH_OWNER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!(
                    "cannot find \"CKB_BENCH_OWNER_PRIVKEY\" from environment variables, error: {}",
                    err
                )
            });
            let genesis_block = nodes[0].clone().genesis_block.unwrap();
            let owner = {
                let owner_privkey = Privkey::from_str(&owner_raw_privkey).unwrap_or_else(|err| {
                    prompt_and_exit!(
                        "failed to parse CKB_BENCH_OWNER_PRIVKEY to Privkey, error: {}",
                        err
                    )
                });
                User::new(genesis_block.clone(), Some(owner_privkey))
            };
            let users = {
                let owner_byte32_privkey =
                    Byte32::from_slice(H256::from_str(&owner_raw_privkey).unwrap().as_bytes())
                        .unwrap_or_else(|err| {
                            prompt_and_exit!(
                                "failed to parse CKB_BENCH_OWNER_PRIVKEY to Byte32, error: {}",
                                err
                            )
                        });
                let privkeys = derive_privkeys(owner_byte32_privkey, n_users);
                privkeys
                    .into_iter()
                    .map(|privkey| User::new(genesis_block.clone(), Some(privkey)))
                    .collect::<Vec<_>>()
            };
            dispatch(&nodes, &owner, &users, cells_per_user, capacity_per_cell);
        }
        ("collect", Some(arguments)) => {
            let data_dir = value_t_or_exit!(arguments, "data-dir", PathBuf);
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    let port = url.port().unwrap();
                    let host = url.host_str().unwrap();
                    let node_data_dir = data_dir.join(&format!("{}:{}", host, port));
                    ::std::fs::create_dir_all(&node_data_dir).unwrap_or_else(|err| {
                        panic!(
                            "failed to create dir \"{}\", error: {}",
                            node_data_dir.display(),
                            err
                        )
                    });
                    Node::init(url.as_str(), url.as_str())
                })
                .collect::<Vec<_>>();
            let n_users = value_t_or_exit!(arguments, "n-users", usize);
            let owner_raw_privkey = env::var("CKB_BENCH_OWNER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!(
                    "cannot find \"CKB_BENCH_OWNER_PRIVKEY\" from environment variables, error: {}",
                    err
                )
            });
            let genesis_block = nodes[0].clone().genesis_block.unwrap();
            let owner = {
                let owner_privkey = Privkey::from_str(&owner_raw_privkey).unwrap_or_else(|err| {
                    prompt_and_exit!(
                        "failed to parse CKB_BENCH_OWNER_PRIVKEY to Privkey, error: {}",
                        err
                    )
                });
                User::new(genesis_block.clone(), Some(owner_privkey))
            };
            let users = {
                let owner_byte32_privkey =
                    Byte32::from_slice(H256::from_str(&owner_raw_privkey).unwrap().as_bytes())
                        .unwrap_or_else(|err| {
                            prompt_and_exit!(
                                "failed to parse CKB_BENCH_OWNER_PRIVKEY to Byte32, error: {}",
                                err
                            )
                        });
                let privkeys = derive_privkeys(owner_byte32_privkey, n_users);
                privkeys
                    .into_iter()
                    .map(|privkey| User::new(genesis_block.clone(), Some(privkey)))
                    .collect::<Vec<_>>()
            };
            collect(&nodes, &owner, &users);
        }
        ("bench", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let data_dir = value_t_or_exit!(arguments, "data-dir", PathBuf);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    let port = url.port().unwrap();
                    let host = url.host_str().unwrap();
                    let node_data_dir = data_dir.join(&format!("{}:{}", host, port));
                    ::std::fs::create_dir_all(&node_data_dir).unwrap_or_else(|err| {
                        panic!(
                            "failed to create dir \"{}\", error: {}",
                            node_data_dir.display(),
                            err
                        )
                    });
                    Node::init(url.as_str(), url.as_str())
                })
                .collect::<Vec<_>>();
            let n_users = value_t_or_exit!(arguments, "n-users", usize);
            let n_inout = value_t_or_exit!(arguments, "n-inout", usize);
            let t_tx_interval = {
                let tx_interval_ms = value_t_or_exit!(arguments, "tx-interval-ms", u64);
                Duration::from_millis(tx_interval_ms)
            };
            let t_bench = {
                let bench_time_ms = value_t_or_exit!(arguments, "bench-time-ms", u64);
                Duration::from_millis(bench_time_ms)
            };
            let owner_raw_privkey = env::var("CKB_BENCH_OWNER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!(
                    "cannot find \"CKB_BENCH_OWNER_PRIVKEY\" from environment variables, error: {}",
                    err
                )
            });
            let genesis_block = nodes[0].clone().genesis_block.unwrap();
            let users = {
                let owner_byte32_privkey =
                    Byte32::from_slice(H256::from_str(&owner_raw_privkey).unwrap().as_bytes())
                        .unwrap_or_else(|err| {
                            prompt_and_exit!(
                                "failed to parse CKB_BENCH_OWNER_PRIVKEY to Byte32, error: {}",
                                err
                            )
                        });
                let privkeys = derive_privkeys(owner_byte32_privkey, n_users);
                privkeys
                    .into_iter()
                    .map(|privkey| User::new(genesis_block.clone(), Some(privkey)))
                    .collect::<Vec<_>>()
            };
            let is_smoking_test = arguments.is_present("is-smoking-test");
            let bench_concurrent_requests_number = value_t_or_exit!(arguments, "concurrent-requests", usize);
            let (live_cell_sender, live_cell_receiver) = bounded(10000000);
            let (transaction_sender, transaction_receiver) = bounded(1000000);

            crate::info!(
                "bench with params --n-users {} --n-inout {} --tx-interval-ms {} --bench-time-ms {} --concurrent-requests {}",
                users.len(), n_inout, t_tx_interval.as_millis(), t_bench.as_millis(),bench_concurrent_requests_number
            );

            let live_cell_producer = LiveCellProducer::new(users.clone(), nodes.clone());
            spawn(move || {
                live_cell_producer.run(live_cell_sender, 3);
            });


            let transaction_producer = TransactionProducer::new(
                users.clone(),
                vec![users[0].single_secp256k1_cell_dep()],
                n_inout,
                AddTxParam::new()
            );
            spawn(move || {
                transaction_producer.run(live_cell_receiver, transaction_sender, 3);
            });

            let watcher = Watcher::new(nodes.clone().into());
            if !is_smoking_test {
                while !watcher.is_zero_load() {
                    sleep(Duration::from_secs(10));
                    crate::info!(
                        "[Watcher] is waiting the node become zero-load, fixed_tip_number: {}",
                        watcher.get_fixed_header().inner.number.value()
                    );
                }
            }

            let zero_load_number = watcher.get_fixed_header().inner.number;
            let rt = Runtime::new().unwrap();
            let tx_consumer = TransactionConsumer::new(nodes.clone());
            crate::info!("---- tx_consumer------");

            rt.block_on(

                tx_consumer.run(transaction_receiver, bench_concurrent_requests_number, t_tx_interval, t_bench)
            );
            if !is_smoking_test {
                while !watcher.is_zero_load() {
                    sleep(Duration::from_secs(10));
                    crate::info!(
                        "[Watcher] is waiting the node become zero-load, fixed_tip_number: {}",
                        watcher.get_fixed_header().inner.number.value()
                    );
                }
            }

            let t_stat = t_bench.div(2);
            let fixed_tip_number = watcher.get_fixed_header().inner.number;
            let report = stat::stat(
                &nodes[0],
                (zero_load_number.value() + 1).into(),
                fixed_tip_number.into(),
                t_stat,
                Some(t_tx_interval),
            );
            crate::info!(
                "markdown report: | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                report.ckb_version,
                report.transactions_per_second,
                report.n_inout,
                report.n_nodes,
                report.delay_time_ms.expect("bench specify delay_time_ms"),
                report.average_block_time_ms,
                report.average_block_transactions,
                report.average_block_transactions_size,
                report.from_block_number,
                report.to_block_number,
                report.total_transactions,
                report.total_transactions_size,
                report.transactions_size_per_second,
            );
            crate::info!("metrics: {}", serde_json::json!(report));
        }
        ("stat", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let from_number = value_t_or_exit!(arguments, "from-number", BlockNumber);
            let to_number = value_t_or_exit!(arguments, "to-number", BlockNumber);
            let stat_time_ms = value_t_or_exit!(arguments, "stat-period-ms", u64);
            let t_stat = Duration::from_millis(stat_time_ms);
            let node = Node::init(rpc_urls[0].as_str(), rpc_urls[0].as_str());
            let report = stat::stat(&node, from_number, to_number, t_stat, None);
            crate::info!("metrics: {}", serde_json::json!(report));
        }
        _ => {
            eprintln!("wrong usage");
            exit(1);
        }
    }
}

fn clap_app() -> App<'static, 'static> {
    include_str!("../Cargo.toml");
    App::new("ckb-bench")
        .version(git_version::git_version!())
        .subcommand(
            SubCommand::with_name("miner")
                .about("runs ckb miner")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .long_help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n-blocks")
                        .short("b")
                        .long("n-blocks")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("How many blocks to mine, 0 means infinitely")
                        .default_value("0")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("mining-interval-ms")
                        .long("mining-interval-ms")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("How long it takes to mine a block.\nNote that it is different with \"block time interval\", we can/should not control the block time interval")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ),
        )
        .subcommand(
            SubCommand::with_name("bench")
                .about("bench the target ckb nodes")
                .arg(
                    Arg::with_name("data-dir")
                        .long("data-dir")
                        .required(true)
                        .takes_value(true)
                        .value_name("PATH")
                        .default_value("./data")
                        .help("Data directory"),
                )
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n-users")
                        .long("n-users")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Number of users")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n-inout")
                        .long("n-inout")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("input-output pairs of a transaction")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("tx-interval-ms")
                        .long("tx-interval-ms")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("Interval of sending transactions in milliseconds")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("bench-time-ms")
                        .long("bench-time-ms")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("Bench time period")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("is-smoking-test")
                        .long("is-smoking-test")
                        .help("Whether the target network is production network, like mainnet, testnet, devnet"),
                )
                .arg(
                    Arg::with_name("concurrent-requests")
                        .long("concurrent-requests")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .default_value("1")
                        .help("Bench concurrent requests")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ),
        )
        .subcommand(
            SubCommand::with_name("dispatch")
                .about("dispatch capacity to users")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n-users")
                        .long("n-users")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Number of users")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("cells-per-user")
                        .long("cells-per-user")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Cells per user")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("capacity-per-cell")
                        .long("capacity-per-cell")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Capacity per cell")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("data-dir")
                        .long("data-dir")
                        .required(true)
                        .takes_value(true)
                        .value_name("PATH")
                        .default_value("./data")
                        .help("Data directory"),
                )
        )
        .subcommand(
            SubCommand::with_name("collect")
                .about("collect capacity back to owner")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n-users")
                        .long("n-users")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("Number of users")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("data-dir")
                        .long("data-dir")
                        .required(true)
                        .takes_value(true)
                        .value_name("PATH")
                        .default_value("./data")
                        .help("Data directory"),
                ),
        )
        .subcommand(
            SubCommand::with_name("stat")
                .about("report chain stat")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .long_help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("from-number")
                        .long("from-number")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("From block number")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("to-number")
                        .long("to-number")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("To block number")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("stat-period-ms")
                        .long("stat-period-ms")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("Stat period")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ),
        )
}

fn init_logger() -> ckb_logger_service::LoggerInitGuard {
    let filter = match env::var("RUST_LOG") {
        Ok(filter) if filter.is_empty() => Some("info".to_string()),
        Ok(filter) => Some(filter.to_string()),
        Err(_) => Some("info".to_string()),
    };
    let config = ckb_logger_config::Config {
        filter,
        color: false,
        log_to_file: false,
        log_to_stdout: true,
        ..Default::default()
    };
    ckb_logger_service::init(None, config)
        .unwrap_or_else(|err| panic!("failed to init the logger service, error: {}", err))
}

//
//
// mod logger;
// mod  node;
// mod nodes;
// pub mod util;
// mod watcher;
// mod utils;
// mod user;
// mod stat;
// mod prepare;
// mod bench;
//
// fn main() {
//     let _logger = init_logger();
//     // use ckb_sdk::rpc::CkbRpcClient;
//     //
//     // let mut ckb_client = CkbRpcClient::new("https://testnet.ckb.dev");
//     // let block = ckb_client.get_block_by_number(0.into()).unwrap();
//     // println!("block: {}", serde_json::to_string_pretty(&block).unwrap());
//     let mut node = Node::init("https://testnet.ckb.dev", "https://testnet.ckb.dev");
//     let block = node.rpc_client.get_block_by_number(0.into()).unwrap();
//     // println!("block: {}", serde_json::to_string_pretty(&block).unwrap());
//
//
// }
//
//
// fn init_logger() -> ckb_logger_service::LoggerInitGuard {
//     let filter = match env::var("RUST_LOG") {
//         Ok(filter) if filter.is_empty() => Some("info".to_string()),
//         Ok(filter) => Some(filter.to_string()),
//         Err(_) => Some("info".to_string()),
//     };
//     let config = ckb_logger_config::Config {
//         filter,
//         color: false,
//         log_to_file: false,
//         log_to_stdout: true,
//         ..Default::default()
//     };
//     ckb_logger_service::init(None, config)
//         .unwrap_or_else(|err| panic!("failed to init the logger service, error: {}", err))
// }