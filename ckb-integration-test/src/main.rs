pub mod case;
pub mod prelude;
pub mod testdata;
pub mod util;

use clap::{value_t_or_exit, App, Arg, ArgMatches, SubCommand};
use lazy_static::lazy_static;
use std::env;
use std::path::PathBuf;
use std::process::exit;
use std::sync::RwLock;

// TODO Create a shortcut for CKB2019/CKB2021
lazy_static! {
    pub static ref CKB2019: RwLock<PathBuf> = RwLock::new(PathBuf::new());
    pub static ref CKB2021: RwLock<PathBuf> = RwLock::new(PathBuf::new());
}

fn filter_cases(arg_matches: &ArgMatches) -> Vec<Box<dyn case::Case>> {
    if let Some(filtering_cases) = arg_matches.values_of("cases") {
        filtering_cases
            .filter_map(|case_name| {
                for case in crate::case::all_cases() {
                    if case.case_name() == case_name {
                        return Some(case);
                    }
                }
                panic!("unknown case \"{}\"", case_name);
            })
            .collect()
    } else {
        crate::case::all_cases()
    }
}

fn main() {
    env::set_var("RUST_BACKTRACE", "full");
    let matches = clap_app().get_matches();
    let _logger = init_logger(&matches);

    match matches.subcommand() {
        ("run", Some(arg_matches)) => {
            crate::init_ckb_binaries(&arg_matches);
            for case in filter_cases(&arg_matches) {
                crate::case::run_case(case);
            }
        }
        ("generate-testdata", Some(arg_matches)) => {
            crate::init_ckb_binaries(&arg_matches);
            let testdatas = crate::testdata::all_testdata_generators();
            for testdata in testdatas {
                testdata.generate();
            }
        }
        _ => {
            println!("invalid usage");
            exit(1);
        }
    }
}

fn clap_app() -> App<'static, 'static> {
    App::new("ckb-integration-test")
        .version(git_version::git_version!())
        .arg(
            Arg::with_name("trace")
                .long("trace")
                .required(false)
                .takes_value(false)
                .conflicts_with("info")
                .conflicts_with("debug")
                .help("Set log.filter=trace")
        )
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .required(false)
                .takes_value(false)
                .conflicts_with("trace")
                .conflicts_with("info")
                .help("Set log.filter=debug")
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Run test cases")
                .arg(
                    Arg::with_name("ckb2019")
                        // hide the help information about `--ckb2019`, we use built-in
                        // ckb2019 binary, located in testdata/bin/
                        .hidden(true)
                        .required(false)
                        .long("ckb2019")
                        .takes_value(true)
                        .value_name("PATH")
                        .help("Path to ckb2019 executable"),
                )
                .arg(
                    Arg::with_name("ckb2021")
                        .long("ckb2021")
                        .takes_value(true)
                        .value_name("PATH")
                        .help("Path to ckb2021 executable"),
                )
                .arg(
                    Arg::with_name("cases")
                        .required(false)
                        .long("cases")
                        .takes_value(true)
                        .multiple(true)
                        .value_name("CASE_NAME")
                        .help("Only run specified cases. Run all cases if this parameter is not setting"),
                )
        )
        .subcommand(
            SubCommand::with_name("generate-testdata")
                .about("Run testdata generators")
                .arg(
                    Arg::with_name("ckb2019")
                        // hide the help information about `--ckb2019`, we use built-in
                        // ckb2019 binary, located in testdata/bin/
                        .hidden(true)
                        .required(false)
                        .long("ckb2019")
                        .takes_value(true)
                        .value_name("PATH")
                        .help("Path to ckb2019 executable"),
                )
                .arg(
                    Arg::with_name("ckb2021")
                        .long("ckb2021")
                        .takes_value(true)
                        .value_name("PATH")
                        .required(false)
                        .help("Path to ckb2021 executable"),
                )
        )
}

fn init_logger(clap_matches: &ArgMatches) -> ckb_logger_service::LoggerInitGuard {
    let filter = if clap_matches.is_present("debug") {
        "debug"
    } else if clap_matches.is_present("trace") {
        "trace"
    } else {
        "info"
    };
    let config = ckb_logger_config::Config {
        filter: Some(filter.to_string()),
        log_to_file: false,
        log_to_stdout: true,
        ..Default::default()
    };
    ckb_logger_service::init(None, config)
        .unwrap_or_else(|err| panic!("failed to init the logger service, error: {}", err))
}

fn init_ckb_binaries(matches: &ArgMatches) {
    let ckb2019 = {
        if let Some(ckb2019_str) = matches.value_of("ckb2019") {
            PathBuf::from(ckb2019_str)
        } else {
            // Use default ckb_v0.43.2 binary according to the running system
            match os_info::get().os_type() {
                os_info::Type::Macos => PathBuf::from("testdata/bin/ckb_v0.43.2-macOS"),
                os_info::Type::Windows => PathBuf::from("testdata/bin/ckb_v0.43.2-Windows"),
                _ => PathBuf::from("testdata/bin/ckb_v0.43.2-Linux"),
            }
        }
    };
    let ckb2021 = value_t_or_exit!(matches, "ckb2021", PathBuf);
    if !ckb2019.exists() || !ckb2019.is_file() {
        panic!("--ckb2019 points to non-executable")
    }
    if !ckb2021.exists() || !ckb2021.is_file() {
        panic!("--ckb2021 points to non-executable")
    }
    *CKB2019.write().unwrap() = absolutize(ckb2019);
    *CKB2021.write().unwrap() = absolutize(ckb2021);
}

fn absolutize(path: PathBuf) -> PathBuf {
    if path.is_relative() {
        env::current_dir()
            .expect("getting current dir should be ok")
            .join(path)
    } else {
        path
    }
}
