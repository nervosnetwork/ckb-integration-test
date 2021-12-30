use crate::prelude::*;
use lazy_static::lazy_static;

lazy_static! {
    static ref V0_43: String = {
        let stdout = ::std::process::Command::new(CKB2019.read().unwrap().clone())
            .arg("--version")
            .output()
            .expect("failed to execute process")
            .stdout;
        String::from_utf8(stdout).unwrap()
    };
    static ref V0_100: String = {
        let stdout = ::std::process::Command::new(CKB2021.read().unwrap().clone())
            .arg("--version")
            .output()
            .expect("failed to execute process")
            .stdout;
        String::from_utf8(stdout).unwrap()
    };
}

pub fn v0_43() -> String {
    V0_43.clone()
}

pub fn v0_100() -> String {
    V0_100.clone()
}
