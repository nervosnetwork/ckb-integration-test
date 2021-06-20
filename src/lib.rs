pub mod case;
pub mod node;
pub mod nodes;
pub mod rpc;
pub mod testdata;
pub mod util;

use ckb_util::Mutex;
use clap::{value_t, ArgMatches};
use lazy_static::lazy_static;
use std::env::current_dir;
use std::path::PathBuf;

lazy_static! {
    pub static ref CKB_V1_BINARY: Mutex<PathBuf> = Mutex::new(PathBuf::new());
    pub static ref CKB_V2_BINARY: Mutex<PathBuf> = Mutex::new(PathBuf::new());
    pub static ref TESTDATA_DIR: Mutex<PathBuf> = Mutex::new(PathBuf::new());
}

pub fn init_ckb_binaries(matches: &ArgMatches) {
    let ckb_v1_binary = value_t!(matches, "ckb-v1-binary", PathBuf)
        .unwrap_or_else(|err| panic!("failed to parse --ckb-v1-binary, error: {}", err));
    let ckb_v2_binary = value_t!(matches, "ckb-v2-binary", PathBuf)
        .unwrap_or_else(|err| panic!("failed to parse --ckb-v2-binary, error: {}", err));
    if !ckb_v1_binary.exists() || !ckb_v1_binary.is_file() {
        panic!("--ckb-v1-binary points to non-executable")
    }
    if !ckb_v2_binary.exists() || !ckb_v2_binary.is_file() {
        panic!("--ckb-v2-binary points to non-executable")
    }
    *CKB_V1_BINARY.lock() = absolutize(ckb_v1_binary);
    *CKB_V2_BINARY.lock() = absolutize(ckb_v2_binary);
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
