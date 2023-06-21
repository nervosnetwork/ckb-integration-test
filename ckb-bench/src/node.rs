// use ckb_jsonrpc_types::BlockView;
// node is  rpc implement
use ckb_sdk::rpc::CkbRpcClient;
use std::time::{Duration, Instant};
use std::thread::sleep;
use ckb_jsonrpc_types::{BlockNumber, BlockView};
use ckb_sdk::rpc::ckb_indexer::{Cell, Order, Pagination, ScriptSearchMode, ScriptType, SearchKey};
use ckb_sdk::RpcError;
use ckb_types::packed;
use ckb_types::packed::Script;
use ckb_types::prelude::{Builder, Entity};
use lazy_static::lazy_static;

use std::collections::HashMap;

pub const MAX_QUERY_CELL_SIZE: u32 = 100000;

use std::sync::{Arc, Mutex};
use crate::rpc::RpcClient;

lazy_static! {
    static ref MAP: Mutex<HashMap<String, Arc<CkbRpcClient>>> = Mutex::new(HashMap::new());
}


pub fn get_or_create_ckb_client(key: String) -> Arc<CkbRpcClient> {
    {
        let map = MAP.lock().unwrap();

        if let Some(value) = map.get(&key) {
            return Arc::clone(value);
        }
    }
    let mut map = MAP.lock().unwrap();
    let default_value = Arc::new(CkbRpcClient::new(key.as_str()));
    let value = map.entry(key).or_insert_with(|| Arc::clone(&default_value));
    Arc::clone(value)
}



#[derive(Debug, Clone, Default)]
pub struct NodeOptions {
    pub node_name: String,
}

pub struct Node {
    pub(super) rpc_client: String,
    //todo Remove async_client : because rust sdk not have async rpc client , blocking
    pub(super) async_client:RpcClient,
    pub(super) indexer: String,
    pub(super) genesis_block: Option<BlockView>,
    // initialize when node start
    pub(super) node_options: NodeOptions,
}

impl Node {
    pub fn init(ckb_rpc_url: &str, ckb_indexer_rpc_rul: &str) -> Self {

        get_or_create_ckb_client(ckb_indexer_rpc_rul.to_string());
        let ckb_client = get_or_create_ckb_client(ckb_rpc_url.to_string());
        let genesis_block = ckb_client.get_block_by_number(0.into()).unwrap();
        let client = RpcClient::new(&ckb_rpc_url);

        let mut node_opt = NodeOptions::default();
        node_opt.node_name = ckb_rpc_url.to_string();

        Self {
            rpc_client: ckb_rpc_url.to_string(),
            indexer: ckb_indexer_rpc_rul.to_string(),
            async_client:client,
            genesis_block,
            node_options: node_opt,
        }
    }
    pub fn node_name(&self) -> &str {
        &self.node_options.node_name
    }
    pub fn rpc_client(&self) -> Arc<CkbRpcClient> {
        get_or_create_ckb_client(self.rpc_client.to_string())
    }

    pub fn async_client(&self) ->&RpcClient{
            &self.async_client
    }

    pub fn get_tip_block(&self) -> BlockView {
        let rpc_client = self.rpc_client();
        let tip_number = rpc_client.get_tip_block_number().unwrap();
        let block = rpc_client
            .get_block_by_number(tip_number)
            .expect("tip block exists");
        crate::trace!(
            "[Node {}] Node::get_tip_block(), block: {:?}",
            self.node_name(),
            block
        );
        block.unwrap()
    }

    pub fn wait_for_tx_pool(&self) {
        let rpc_client = self.rpc_client();
        let mut chain_tip = rpc_client.get_tip_header().unwrap();
        let mut tx_pool_tip = rpc_client.tx_pool_info().unwrap();
        if chain_tip.hash == tx_pool_tip.tip_hash {
            return;
        }
        let mut instant = Instant::now();
        while instant.elapsed() < Duration::from_secs(10) {
            sleep(std::time::Duration::from_secs(1));
            chain_tip = rpc_client.get_tip_header().unwrap();
            let prev_tx_pool_tip = tx_pool_tip;
            tx_pool_tip = rpc_client.tx_pool_info().unwrap();
            if chain_tip.hash == tx_pool_tip.tip_hash {
                return;
            } else if prev_tx_pool_tip.tip_hash != tx_pool_tip.tip_hash
                && tx_pool_tip.tip_number.value() < chain_tip.inner.number.value()
            {
                instant = Instant::now();
            }
        }

        panic!(
            "timeout to wait for tx pool,\n\tchain   tip: {:?}, {:#x},\n\ttx-pool tip: {}, {:#x}",
            chain_tip.inner.number.value(),
            chain_tip.hash,
            tx_pool_tip.tip_number.value(),
            tx_pool_tip.tip_hash,
        );
    }
    pub fn indexer(&self) -> Arc<CkbRpcClient> {
        get_or_create_ckb_client(self.indexer.to_string())
    }

    pub fn get_cells_by_script(&self, script: Script) -> Result<Pagination<Cell>, RpcError> {
        let search_key = SearchKey {
            script: Script::new_builder()
                .code_hash(script.code_hash())
                .hash_type(script.hash_type())
                .args(script.args())
                .build()
                .into(),
            script_type: ScriptType::Lock,
            script_search_mode: Some(ScriptSearchMode::Exact),
            filter: None,
            with_data: None,
            group_by_transaction: None,
        };

        self.indexer().get_cells(search_key, Order::Asc, MAX_QUERY_CELL_SIZE.into(), None)
        //TODO : check last_cursor is none ?
    }

    pub fn mine(&self, n_blocks: u64) {
        for _ in 0..n_blocks {
            let template = self.rpc_client().get_block_template(None, None, None).unwrap();
            let block = packed::Block::from(template);
            self.rpc_client().submit_block("".into(), block.into()).unwrap();
            self.wait_for_tx_pool();
        }
    }

    pub fn mine_to(&self, target_height: BlockNumber) {
        let tip_number = self.rpc_client().get_tip_block_number().unwrap();
        if tip_number.value() < target_height.value() {
            let n_blocks = target_height.value() - tip_number.value();
            self.mine(n_blocks.into());
        }
    }
}

impl Clone for Node {
    fn clone(&self) -> Node {
        Self {
            node_options: self.node_options.clone(),
            rpc_client: self.rpc_client.to_string(),
            async_client: self.async_client.clone(),
            genesis_block: self.genesis_block.clone(),
            indexer: self.indexer.to_string(),
        }
    }
}