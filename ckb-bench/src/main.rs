mod config;
mod user;

use ckb_testkit::{node::Node, nodes::Nodes};
use clap::{value_t_or_exit, values_t_or_exit, App, Arg, ArgMatches, SubCommand};
use config::Url;
use std::env;
use std::path::PathBuf;
use std::process::exit;
use std::thread::{sleep, spawn};
use std::time::Duration;
use user::User;

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
            let node = Node::init_from_url(&rpc_urls[0], Default::default());
            node.mine(n_blocks);
        }
        ("bench", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let spec_path = value_t_or_exit!(arguments, "spec", PathBuf);
            let spec = config::Spec::load(&spec_path);
            let skip_best_tps_calculation = arguments.is_present("skip-best-tps-calculation");

            if let Some(ref miner_config) = spec.miner {
                let block_time = miner_config.block_time;
                let nodes_ = rpc_urls
                    .iter()
                    .map(|url| Node::init_from_url(url, Default::default()))
                    .collect::<Vec<_>>();
                let miners = Nodes::from(nodes_);
                spawn(move || loop {
                    for miner in miners.nodes() {
                        miner.mine(1);
                        sleep(Duration::from_millis(block_time));
                    }
                });
            }
            let nodes = {
                let nodes_ = rpc_urls
                    .iter()
                    .map(|url| {
                        let node_working_dir = spec.working_dir.join(&url.to_string());
                        Node::init_from_url(url, node_working_dir)
                    })
                    .collect::<Vec<_>>();
                Nodes::from(nodes_)
            };
            let users = spec
                .users
                .iter()
                .map(|privkey| User::new(privkey))
                .collect::<Vec<_>>();
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
                        .help("number of blocks to mine")
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
                )
                .arg(
                    Arg::with_name("skip-best-tps-calculation")
                        .help("whether skip best tps calculation"),
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
