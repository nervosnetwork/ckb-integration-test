use lazy_static::lazy_static;
use std::env;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::path::PathBuf;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering::SeqCst;
use std::thread::sleep;
use std::time::{Duration, Instant};

lazy_static! {
    pub static ref PORT_COUNTER: AtomicU16 = AtomicU16::new(9000);
}

pub fn find_available_port() -> u16 {
    for _ in 0..2000 {
        let port = PORT_COUNTER.fetch_add(1, SeqCst);
        let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
        if TcpListener::bind(address).is_ok() {
            return port;
        }
    }
    panic!("failed to allocate available port")
}

/// Return a random path located on temp_dir
///
/// We use `tempdir` only for generating a random path, and expect the corresponding directory
/// that `tempdir` creates be deleted when go out of this function.
pub fn temp_path(case_name: &str, suffix: &str) -> PathBuf {
    let mut builder = tempfile::Builder::new();
    let prefix = ["ckb-it", case_name, suffix, ""].join("-");
    builder.prefix(&prefix);
    let tempdir = if let Ok(val) = env::var("CKB_INTEGRATION_TEST_TMP") {
        builder.tempdir_in(val)
    } else {
        builder.tempdir()
    }
    .expect("create tempdir failed");
    let path = tempdir.path().to_owned();
    tempdir.close().expect("close tempdir failed");
    path
}

pub fn wait_until<F>(timeout_secs: u64, mut f: F) -> bool
where
    F: FnMut() -> bool,
{
    let timeout = Duration::from_secs(timeout_secs);
    let start = Instant::now();
    while Instant::now().duration_since(start) <= timeout {
        if f() {
            return true;
        }
        sleep(Duration::new(1, 0));
    }
    false
}
