use crate::error;
use crate::rpc::RpcClient;
use crate::util::{find_available_port, temp_path};
use crate::NodeOptions;
use ckb_indexer::{
    indexer::Indexer,
    store::{RocksdbStore, Store},
};
use ckb_jsonrpc_types::{Consensus, LocalNode};
use ckb_types::core::BlockView;
use fs_extra::dir::CopyOptions;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

struct ProcessGuard(pub Child);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _x = self
            .0
            .kill()
            .map_err(|err| error!("failed to kill ckb process, error: {}", err));
        let _y = self.0.wait();
    }
}

pub struct Node {
    pub(super) node_options: NodeOptions,

    pub(super) working_dir: PathBuf,
    pub(super) rpc_client: RpcClient,
    pub(super) p2p_listen: String,

    pub(super) consensus: Option<Consensus>, // initialize when node start
    pub(super) genesis_block: Option<BlockView>, // initialize when node start
    pub(super) node_id: Option<String>,      // initialize when node start
    pub(super) indexer: Option<Indexer<RocksdbStore>>, // initialize when node start
    _guard: Option<ProcessGuard>,            // initialize when node start
}

impl Clone for Node {
    fn clone(&self) -> Node {
        Self {
            node_options: self.node_options.clone(),
            working_dir: self.working_dir().clone(),
            rpc_client: self.rpc_client.clone(),
            p2p_listen: self.p2p_listen.clone(),
            consensus: self.consensus.clone(),
            genesis_block: self.genesis_block.clone(),
            node_id: self.node_id.clone(),
            indexer: self.indexer.clone(),
            _guard: None,
        }
    }
}

impl Node {
    pub fn init<S: ToString>(case_name: S, node_options: NodeOptions, is_ckb2021: bool) -> Self {
        let case_name = case_name.to_string();
        let rpc_port = find_available_port();
        let p2p_port = find_available_port();
        let working_dir = prepare_working_dir(&case_name, &node_options, rpc_port, p2p_port);
        Self {
            node_options,
            working_dir,
            rpc_client: RpcClient::new(&format!("http://127.0.0.1:{}/", rpc_port), is_ckb2021),
            p2p_listen: format!("/ip4/0.0.0.0/tcp/{}", p2p_port),
            consensus: None,
            genesis_block: None,
            node_id: None,
            indexer: None,
            _guard: None,
        }
    }

    pub fn init_from_url(rpc_url: &str, working_dir: PathBuf) -> Self {
        crate::info!("init via \"{}\"", rpc_url);
        let mut rpc_client = RpcClient::new(rpc_url, true);
        let local_node_info = rpc_client.local_node_info();
        let is_ckb2021 = {
            let node_version = &local_node_info.version;
            let minimal_2021_version = "0.44.0";
            let is_ckb2021 = version_compare::VersionCompare::compare_to(
                node_version,
                minimal_2021_version,
                &version_compare::CompOp::Ge,
            )
            .unwrap_or(true);
            is_ckb2021
        };
        if !is_ckb2021 {
            rpc_client = RpcClient::new(&rpc_url, is_ckb2021)
        }

        let consensus = rpc_client.get_consensus();
        let genesis_block = rpc_client
            .get_block_by_number(0)
            .expect("get genesis block");
        let node_id = local_node_info.node_id.to_owned();
        let indexer = if working_dir.to_string_lossy().is_empty() {
            None
        } else {
            let data_path = working_dir.join("indexer");
            let store = RocksdbStore::new(&data_path.to_string_lossy());
            Some(Indexer::new(store, 1000000, 60 * 60))
        };
        Self {
            // TODO get p2p listen address via RPC
            node_options: Default::default(),
            working_dir,
            rpc_client,
            p2p_listen: Default::default(),
            consensus: Some(consensus),
            genesis_block: Some(genesis_block.into()),
            node_id: Some(node_id),
            indexer,
            _guard: None,
        }
    }

    pub fn start(&mut self) {
        let binary = &self.node_options.ckb_binary;
        let mut child_process = Command::new(&binary)
            .env("RUST_BACKTRACE", "full")
            .args(&[
                "-C",
                &self.working_dir().to_string_lossy().to_string(),
                "run",
                "--ba-advanced",
                "--overwrite-spec",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|err| {
                panic!(
                    "failed to start ckb process, binary: {}, error: {}",
                    binary.display(),
                    err
                )
            });
        let local_node_info = self.wait_for_node_up(&mut child_process);
        let consensus = self.rpc_client().get_consensus();
        let genesis_block = self.get_block_by_number(0);
        let indexer = {
            let data_path = self.working_dir().join("indexer");
            let store = RocksdbStore::new(&data_path.to_string_lossy());
            Indexer::new(store, 1000000, 60 * 60)
        };

        self.consensus = Some(consensus);
        self.genesis_block = Some(genesis_block);
        self._guard = Some(ProcessGuard(child_process));
        self.node_id = Some(local_node_info.node_id);
        self.indexer = Some(indexer);
        crate::info!(
            "[Node {}] START node_id: \"{}\", p2p_listen: \"{}\", log_path: \"{}\"",
            self.node_name(),
            self.node_id(),
            self.p2p_listen,
            self.log_path().display()
        );
    }

    pub fn node_name(&self) -> &str {
        &self.node_options.node_name
    }

    pub fn node_options(&self) -> &NodeOptions {
        &self.node_options
    }

    pub fn working_dir(&self) -> PathBuf {
        self.working_dir.clone()
    }

    pub fn log_path(&self) -> PathBuf {
        self.working_dir().join("data/logs/run.log")
    }

    pub fn rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }

    pub fn p2p_listen(&self) -> String {
        self.p2p_listen.clone()
    }

    pub fn p2p_address(&self) -> String {
        format!("{}/p2p/{}", self.p2p_listen(), self.node_id())
    }

    pub fn consensus(&self) -> &Consensus {
        self.consensus.as_ref().expect("uninitialized consensus")
    }

    pub fn genesis_block(&self) -> &BlockView {
        self.genesis_block
            .as_ref()
            .expect("uninitialized genesis_block")
    }

    pub fn node_id(&self) -> &str {
        // peer_id.to_base58()
        self.node_id.as_ref().expect("uninitialized node_id")
    }

    pub fn indexer(&self) -> &Indexer<RocksdbStore> {
        self.wait_for_indexer_synced();
        self.indexer.as_ref().expect("uninitialized indexer")
    }

    pub fn stop(&mut self) {
        crate::info!(
            "[Node {}] STOP log_path: {}",
            self.node_name(),
            self.log_path().display(),
        );
        if self._guard.is_some() {
            drop(self._guard.take())
        }
    }

    fn wait_for_node_up(&self, child_process: &mut Child) -> LocalNode {
        let start_time = Instant::now();
        while start_time.elapsed() <= Duration::from_secs(60) {
            if let Ok(local_node_info) = self.rpc_client().inner().local_node_info() {
                let _x = self.rpc_client().tx_pool_info();
                return local_node_info;
            }
            match child_process.try_wait() {
                Ok(None) => sleep(std::time::Duration::from_secs(1)),
                Ok(Some(status)) => {
                    error!(
                        "{} node crashed, {}, log_path: {}",
                        self.node_name(),
                        status,
                        self.log_path().display()
                    );
                    process::exit(status.code().unwrap());
                }
                Err(error) => {
                    error!(
                        "{} node crashed with reason: {}, log_path: {}",
                        self.node_name(),
                        error,
                        self.log_path().display()
                    );
                    process::exit(255);
                }
            }
        }
        panic!("timeout to start node process")
    }
}

fn prepare_working_dir(
    case_name: &str,
    node_options: &NodeOptions,
    rpc_port: u16,
    p2p_port: u16,
) -> PathBuf {
    let working_dir: PathBuf = temp_path(&case_name, &node_options.node_name);
    let target_database = &working_dir.join("data/db");
    let source_database = node_options.initial_database;
    let source_chain_spec = node_options.chain_spec;
    let source_app_config = node_options.app_config;

    fs::create_dir_all(target_database).unwrap_or_else(|err| {
        panic!(
            "failed to create dir \"{}\", error: {}",
            target_database.display(),
            err
        )
    });
    fs_extra::dir::copy(
        source_database,
        target_database,
        &CopyOptions {
            content_only: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|err| {
        panic!(
            "failed to copy {} to {}, error: {}",
            source_database,
            target_database.display(),
            err
        )
    });
    fs_extra::dir::copy(
        source_chain_spec,
        &working_dir,
        &CopyOptions {
            content_only: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|err| {
        panic!(
            "failed to copy {} to {}, error: {}",
            source_chain_spec,
            working_dir.display(),
            err
        )
    });
    fs_extra::dir::copy(
        source_app_config,
        &working_dir,
        &CopyOptions {
            content_only: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|err| {
        panic!(
            "failed to copy {} to {}, error: {}",
            source_app_config,
            working_dir.display(),
            err
        )
    });

    // Modify rpc port and p2p port in ckb.toml
    let app_config = working_dir.join("ckb.toml");
    let content = fs::read_to_string(&app_config)
        .unwrap_or_else(|err| panic!("failed to read {}, error: {}", app_config.display(), err));
    let content = content
        .replace("__RPC_PORT__", &rpc_port.to_string())
        .replace("__P2P_PORT__", &p2p_port.to_string());
    fs::write(&app_config, content)
        .unwrap_or_else(|err| panic!("failed to write {}, error: {}", app_config.display(), err));

    working_dir
}
