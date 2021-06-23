pub mod case;
pub mod logger;
pub mod node;
pub mod nodes;
pub mod rpc;
pub mod testdata;
pub mod util;

use ckb_util::Mutex;
use clap::{value_t, ArgMatches};
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::env::current_dir;
use std::path::PathBuf;

thread_local! {
    pub static CASE_NAME: RefCell<String> = RefCell::new(String::new());
}

lazy_static! {
    pub static ref CKB_FORK0_BINARY: Mutex<PathBuf> = Mutex::new(PathBuf::new());
    pub static ref CKB_FORK2021_BINARY: Mutex<PathBuf> = Mutex::new(PathBuf::new());
    pub static ref TESTDATA_DIR: Mutex<PathBuf> = Mutex::new(PathBuf::new());
}

pub fn init_ckb_binaries(matches: &ArgMatches) {
    let ckb_fork0_binary = value_t!(matches, "ckb-fork0-binary", PathBuf)
        .unwrap_or_else(|err| panic!("failed to parse --ckb-fork0-binary, error: {}", err));
    let ckb_fork2021_binary = value_t!(matches, "ckb-fork2021-binary", PathBuf)
        .unwrap_or_else(|err| panic!("failed to parse --ckb-fork2021-binary, error: {}", err));
    if !ckb_fork0_binary.exists() || !ckb_fork0_binary.is_file() {
        panic!("--ckb-fork0-binary points to non-executable")
    }
    if !ckb_fork2021_binary.exists() || !ckb_fork2021_binary.is_file() {
        panic!("--ckb-fork2021-binary points to non-executable")
    }
    *CKB_FORK0_BINARY.lock() = absolutize(ckb_fork0_binary);
    *CKB_FORK2021_BINARY.lock() = absolutize(ckb_fork2021_binary);
}

pub fn init_testdata_dir(matches: &ArgMatches) {
    let testdata_dir = value_t!(matches, "testdata-dir", PathBuf)
        .unwrap_or_else(|err| panic!("failed to parse --testdata-dir, error: {}", err));
    *TESTDATA_DIR.lock() = absolutize(testdata_dir);
}

fn absolutize(path: PathBuf) -> PathBuf {
    if path.is_relative() {
        current_dir()
            .expect("getting current dir should be ok")
            .join(path)
    } else {
        path
    }
}
