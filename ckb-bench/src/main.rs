mod config;
mod prepare;

use crate::prepare::{collect, dispatch, generate_privkeys};
use ckb_crypto::secp::Privkey;
use ckb_testkit::{Node, User};
use ckb_types::{packed::Byte32, prelude::*};
use clap::{value_t_or_exit, values_t_or_exit, App, Arg, ArgMatches, SubCommand};
use config::Url;
use std::env;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

#[macro_export]
macro_rules! prompt_and_exit {
    ($($arg:tt)*) => ({
        eprintln!($($arg)*);
        ckb_testkit::error!($($arg)*);
        ::std::process::exit(1);
    })
}

fn main() {
    let _ = init_logger();
    match clap_app().subcommand() {
        ("mine", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let n_blocks = value_t_or_exit!(arguments, "blocks", u64);
            let block_time_millis = value_t_or_exit!(arguments, "block_time_millis", u64);
            let miners = rpc_urls
                .iter()
                .map(|url| Node::init_from_url(url, Default::default()))
                .collect::<Vec<_>>();
            let mut mined_n_blocks = 0;
            loop {
                for miner in miners.iter() {
                    miner.mine(1);
                    if n_blocks != 0 {
                        mined_n_blocks += 1;
                    }
                    if block_time_millis != 0 {
                        sleep(Duration::from_millis(block_time_millis));
                    }
                }
                if mined_n_blocks > n_blocks {
                    break;
                }
            }
        }
        ("dispatch", Some(arguments)) => {
            let spec_path = value_t_or_exit!(arguments, "spec", PathBuf);
            let spec = config::Spec::load(&spec_path);
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    let node_working_dir = spec.working_dir.join(&url.to_string());
                    Node::init_from_url(url, node_working_dir)
                })
                .collect::<Vec<_>>();
            let n_borrowers = value_t_or_exit!(arguments, "n_borrowers", usize);
            let borrow_capacity = value_t_or_exit!(arguments, "borrow_capacity", u64);
            let lender_raw_privkey = env::var("CKB_BENCH_LENDER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!("cannot find \"CKB_BENCH_LENDER_PRIVKEY\" from environment variables, error: {}", err)
            });
            let lender = {
                let lender_privkey = Privkey::from_str(&lender_raw_privkey).unwrap_or_else(|err| {
                    prompt_and_exit!(
                        "failed to parse CKB_BENCH_LENDER_PRIVKEY to Privkey, error: {}",
                        err
                    )
                });
                User::new(nodes[0].get_block_by_number(0), Some(lender_privkey))
            };
            let borrowers = {
                let lender_byte32_privkey = Byte32::from_slice(lender_raw_privkey.as_bytes())
                    .unwrap_or_else(|err| {
                        prompt_and_exit!(
                            "failed to parse CKB_BENCH_LENDER_PRIVKEY to Byte32, error: {}",
                            err
                        )
                    });
                let privkeys = generate_privkeys(lender_byte32_privkey, n_borrowers);
                privkeys
                    .into_iter()
                    .map(|privkey| User::new(nodes[0].get_block_by_number(0), Some(privkey)))
                    .collect::<Vec<_>>()
            };
            dispatch(&nodes, &lender, &borrowers, borrow_capacity);
        }
        ("collect", Some(arguments)) => {
            let spec_path = value_t_or_exit!(arguments, "spec", PathBuf);
            let spec = config::Spec::load(&spec_path);
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    let node_working_dir = spec.working_dir.join(&url.to_string());
                    Node::init_from_url(url, node_working_dir)
                })
                .collect::<Vec<_>>();
            let n_borrowers = value_t_or_exit!(arguments, "n_borrowers", usize);
            let lender_raw_privkey = env::var("CKB_BENCH_LENDER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!("cannot find \"CKB_BENCH_LENDER_PRIVKEY\" from environment variables, error: {}", err)
            });
            let lender = {
                let lender_privkey = Privkey::from_str(&lender_raw_privkey).unwrap_or_else(|err| {
                    prompt_and_exit!(
                        "failed to parse CKB_BENCH_LENDER_PRIVKEY to Privkey, error: {}",
                        err
                    )
                });
                User::new(nodes[0].get_block_by_number(0), Some(lender_privkey))
            };
            let borrowers = {
                let lender_byte32_privkey = Byte32::from_slice(lender_raw_privkey.as_bytes())
                    .unwrap_or_else(|err| {
                        prompt_and_exit!(
                            "failed to parse CKB_BENCH_LENDER_PRIVKEY to Byte32, error: {}",
                            err
                        )
                    });
                let privkeys = generate_privkeys(lender_byte32_privkey, n_borrowers);
                privkeys
                    .into_iter()
                    .map(|privkey| User::new(nodes[0].get_block_by_number(0), Some(privkey)))
                    .collect::<Vec<_>>()
            };
            collect(&nodes, &lender, &borrowers);
        }
        ("bench", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let spec_path = value_t_or_exit!(arguments, "spec", PathBuf);
            let spec = config::Spec::load(&spec_path);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    let node_working_dir = spec.working_dir.join(&url.to_string());
                    Node::init_from_url(url, node_working_dir)
                })
                .collect::<Vec<_>>();
            let n_borrowers = value_t_or_exit!(arguments, "n_borrowers", usize);
            let lender_raw_privkey = env::var("CKB_BENCH_LENDER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!("cannot find \"CKB_BENCH_LENDER_PRIVKEY\" from environment variables, error: {}", err)
            });
            let borrowers = {
                let lender_byte32_privkey = Byte32::from_slice(lender_raw_privkey.as_bytes())
                    .unwrap_or_else(|err| {
                        prompt_and_exit!(
                            "failed to parse CKB_BENCH_LENDER_PRIVKEY to Byte32, error: {}",
                            err
                        )
                    });
                let privkeys = generate_privkeys(lender_byte32_privkey, n_borrowers);
                privkeys
                    .into_iter()
                    .map(|privkey| User::new(nodes[0].get_block_by_number(0), Some(privkey)))
                    .collect::<Vec<_>>()
            };
        }
        _ => {
            eprintln!("wrong usage");
            exit(1);
        }
    }
}

fn clap_app() -> ArgMatches<'static> {
    include_str!("../Cargo.toml");
    App::new("ckb-bench")
        .subcommand(
            SubCommand::with_name("mine")
                .about("Mine specified blocks")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("blocks")
                        .short("b")
                        .long("blocks")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("number of blocks to mine, default is infinite(0)")
                        .default_value("0")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("block_time_millis")
                        .long("block_time_millis")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("block time, default is 0")
                        .default_value("0")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ),
        )
        .subcommand(
            SubCommand::with_name("bench")
                .about("bench the target ckb nodes")
                .arg(
                    Arg::with_name("spec")
                        .short("s")
                        .long("spec")
                        .required(true)
                        .takes_value(true)
                        .value_name("PATH")
                        .help("path to spec file"),
                )
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                ),
        )
        .subcommand(
            SubCommand::with_name("dispatch")
                .about("dispatch lender's capacity to borrowers")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n_borrowers")
                        .long("n_borrowers")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("number of borrowers")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ),
        )
        .subcommand(
            SubCommand::with_name("collect")
                .about("collect borrowers' capacity back to lender")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n_borrowers")
                        .long("n_borrowers")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("number of borrowers")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ),
        )
        .get_matches()
}

fn init_logger() -> ckb_logger_service::LoggerInitGuard {
    let filter = match env::var("RUST_LOG") {
        Ok(filter) if filter.is_empty() => Some("info".to_string()),
        Ok(filter) => Some(filter.to_string()),
        Err(_) => Some("info".to_string()),
    };
    let config = ckb_logger_config::Config {
        filter,
        log_to_file: false,
        log_to_stdout: true,
        ..Default::default()
    };
    ckb_logger_service::init(None, config)
        .unwrap_or_else(|err| panic!("failed to init the logger service, error: {}", err))
}
