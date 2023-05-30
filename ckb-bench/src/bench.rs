use crossbeam_channel::{Receiver, Sender};
use lru::LruCache;
use std::collections::HashMap;
use std::sync::{Arc};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use tokio::time::sleep as async_sleep;
use crate::utils::maybe_retry_send_transaction_async;
use std::sync::atomic::{AtomicUsize, Ordering};
use ckb_jsonrpc_types::{OutPoint, CellDep as CellDepJson, Script as ScriptJson,  JsonBytes};
use ckb_types::core::{TransactionBuilder, TransactionView};
use ckb_sdk::rpc::ckb_indexer::Cell;
use ckb_types::core::EpochNumberWithFraction;
use ckb_types::H256;
use ckb_types::packed::{Byte32, ScriptOpt, OutPoint as OutPointByte, CellDep, CellInput, CellOutput, Script};
use ckb_types::prelude::{Builder, Entity, Pack};
use ckb_bench::util::since_from_absolute_epoch_number_with_fraction;
use crate::node::Node;
use crate::user::User;

pub struct LiveCellProducer {
    users: Vec<User>,
    nodes: Vec<Node>,
    seen_out_points: LruCache<OutPoint, Instant>,
}

impl LiveCellProducer {
    pub fn new(users: Vec<User>, nodes: Vec<Node>) -> Self {
        let n_users = users.len();

        let mut user_unused_max_cell_count_cache = 1;
        // step_by: 20 : using a sampling method to find the user who owns the highest number of cells.
        // seen_out_points lruCache cache size = user_unused_max_cell_count_cache * n_users + 10
        // seen_out_points lruCache: preventing unused cells on the chain from being reused.
        for i in (0..=users.len()-1).step_by(20) {
            let user_unused_cell_count_cache = users.get(i).expect("out of bound").get_spendable_single_secp256k1_cells(&nodes[0]).len();
            if user_unused_cell_count_cache > user_unused_max_cell_count_cache && user_unused_cell_count_cache <= 10000 {
                user_unused_max_cell_count_cache = user_unused_cell_count_cache;
            }
            crate::debug!("idx:{}:user_unused_cell_count_cache:{}",i,user_unused_cell_count_cache);
        }
        crate::debug!("user max cell count cache:{}",user_unused_max_cell_count_cache);
        let lrc_cache_size = n_users * user_unused_max_cell_count_cache + 10;
        crate::info!("init unused cache size:{}",lrc_cache_size);
        Self {
            users,
            nodes,
            seen_out_points: LruCache::new(lrc_cache_size),
        }
    }

    pub fn run(mut self, live_cell_sender: Sender<Cell>, log_duration: u64) {
        let mut count = 0;
        let mut start_time = Instant::now();
        let mut duration_count = 0;
        let mut fist_send_finished = true;
        loop {
            let current_loop_start_time = Instant::now();
            let min_tip_number = self
                .nodes
                .iter()
                .map(|node| node.rpc_client().get_tip_block_number().unwrap())
                .min()
                .unwrap();
            for user in self.users.iter() {
                let live_cells = user
                    .get_spendable_single_secp256k1_cells(&self.nodes[0])
                    .into_iter()
                    // TODO reduce competition
                    .filter(|cell| {
                        if self.seen_out_points.contains(&cell.out_point) {
                            return false;
                        }
                       if cell.block_number > min_tip_number {
                            return false;
                        }
                        true
                    })
                    .collect::<Vec<_>>();
                for cell in live_cells {
                    self.seen_out_points
                        .put(cell.out_point.clone().into(), Instant::now());
                    let _ignore = live_cell_sender.send(cell);
                    count += 1;
                    duration_count += 1;
                    if Instant::now().duration_since(start_time) >= Duration::from_secs(log_duration) {
                        let elapsed = start_time.elapsed();
                        crate::info!("[LiveCellProducer] producer count: {} ,duration time:{:?} , duration tps:{}", count,elapsed,duration_count*1000/elapsed.as_millis());
                        duration_count = 0;
                        start_time = Instant::now();
                    }
                }
            }
            if fist_send_finished {
                fist_send_finished = false;
                self.seen_out_points.resize(count + 10)
            }
            crate::debug!("[LiveCellProducer] delay:{:?},total producer:{}",current_loop_start_time.elapsed(),count);
        }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug,Serialize, Deserialize)]
pub struct AddTxParam{
    pub deps:Vec<CellDepJson>,
    pub _type:ScriptJson,
    pub output_data: JsonBytes
}

impl AddTxParam {
    pub(crate) fn get_output_data(&mut self) ->ckb_types::packed::Bytes {
        ckb_types::packed::Bytes::from(self.output_data.clone())
    }
}

impl AddTxParam {
    pub fn new () -> Self {
       Self {
           deps: vec![],
           _type: ScriptJson::default(),
           output_data:Default::default()
       }
    }

    pub fn get_cell_deps(&mut self) -> Vec<CellDep>{
        let mut updated_vec: Vec<CellDep> = Vec::new();
        for item in self.deps.iter() {
            updated_vec.push( CellDep::new_builder()
                .out_point(
                    OutPointByte::new(item.out_point.tx_hash.pack(),item.out_point.index.value())
                ).dep_type(ckb_types::core::DepType::from(item.dep_type.clone()).into())
                .build())
        }
        updated_vec
    }
    pub fn get_script_obj(&mut self) -> ScriptOpt{
        // if self._type
        if self._type.code_hash ==  H256::default() {
            ScriptOpt::default()
        }else {
            Some(Script::new_builder()
                .code_hash(self._type.code_hash.pack())
                .args(self._type.args.clone().into_bytes().pack())
                .hash_type(ckb_types::core::ScriptHashType::from(self._type.hash_type.clone()).into())
                .build()).pack()
        }
    }
}
pub struct TransactionProducer {
    // #{ lock_hash => user }
    users: HashMap<Byte32, User>,
    cell_deps: Vec<CellDep>,
    n_inout: usize,
    // #{ lock_hash => live_cell }
    live_cells: HashMap<Byte32, Cell>,
    // #{ out_point => live_cell }
    backlogs: HashMap<Byte32, Vec<Cell>>,
    add_tx_param: AddTxParam,
}

impl TransactionProducer {
    pub fn new(users: Vec<User>, cell_deps: Vec<CellDep>, n_inout: usize,add_tx_param:AddTxParam) -> Self {
        let mut users_map = HashMap::new();
        for user in users {
            // To support environment `CKB_BENCH_ENABLE_DATA1_SCRIPT`, we have to index 3
            // kinds of cells
            users_map.insert(
                user.single_secp256k1_lock_script_via_type()
                    .calc_script_hash(),
                user.clone(),
            );
            users_map.insert(
                user.single_secp256k1_lock_script_via_data()
                    .calc_script_hash(),
                user.clone(),
            );
            users_map.insert(
                user.single_secp256k1_lock_script_via_data1()
                    .calc_script_hash(),
                user.clone(),
            );
        }

        Self {
            users: users_map,
            cell_deps,
            n_inout,
            live_cells: HashMap::new(),
            backlogs: HashMap::new(),
            add_tx_param,
        }
    }

    pub fn run(
        mut self,
        live_cell_receiver: Receiver<Cell>,
        transaction_sender: Sender<TransactionView>,
        log_duration: u64,
    ) {
        // Environment variables `CKB_BENCH_ENABLE_DATA1_SCRIPT` and
        // `CKB_BENCH_ENABLE_INVALID_SINCE_EPOCH` are temporary.
        let enabled_data1_script = match ::std::env::var("CKB_BENCH_ENABLE_DATA1_SCRIPT") {
            Ok(raw) => {
                raw.parse()
                    .map_err(|err| crate::error!("failed to parse environment variable \"CKB_BENCH_ENABLE_DATA1_SCRIPT={}\", error: {}", raw, err))
                    .unwrap_or(false)
            }
            Err(_) => false,
        };
        let enabled_invalid_since_epoch = match ::std::env::var("CKB_BENCH_ENABLE_INVALID_SINCE_EPOCH") {
            Ok(raw) => {
                raw.parse()
                    .map_err(|err| crate::error!("failed to parse environment variable \"CKB_BENCH_ENABLE_INVALID_SINCE_EPOCH={}\", error: {}", raw, err))
                    .unwrap_or(false)
            }
            Err(_) => false,
        };
        crate::info!("CKB_BENCH_ENABLE_DATA1_SCRIPT = {}", enabled_data1_script);
        crate::info!(
            "CKB_BENCH_ENABLE_INVALID_SINCE_EPOCH = {}",
            enabled_invalid_since_epoch
        );
        let mut count = 0;
        let mut start_time = Instant::now();
        let mut duration_count = 0;


        let mut tx_cell_deps = self.cell_deps.clone();
        tx_cell_deps.extend(self.add_tx_param.get_cell_deps());

        while let Ok(live_cell) = live_cell_receiver.recv() {
            let lock_hash = ckb_types::packed::Script::from(live_cell.output.lock.clone()).calc_script_hash();

            if let Some(_live_cell_in_map) = self.live_cells.get(&lock_hash) {
                self.backlogs
                    .entry(lock_hash.clone())
                    .or_insert_with(Vec::new)
                    .push(live_cell);
            } else {
                self.live_cells.insert(lock_hash.clone(), live_cell);
                for (hash, backlog_cells) in self.backlogs.iter_mut() {
                    if self.live_cells.len() >= self.n_inout {
                        break;
                    }
                    if !self.live_cells.contains_key(hash) && !backlog_cells.is_empty() {
                        if let Some(backlog_cell) = backlog_cells.pop() {
                            self.live_cells.insert(hash.clone(), backlog_cell);
                        }
                    }
                }
            }

            if self.live_cells.len() >= self.n_inout {
                let mut live_cells = HashMap::new();
                std::mem::swap(&mut self.live_cells, &mut live_cells);

                let since = if enabled_invalid_since_epoch {
                    since_from_absolute_epoch_number_with_fraction(
                        EpochNumberWithFraction::new_unchecked(0, 1, 1),
                    )
                } else {
                    0
                };
                let inputs = live_cells
                    .values()
                    .map(|cell| {
                        CellInput::new_builder()
                            .previous_output(cell.out_point.clone().into())
                            .since(since.pack())
                            .build()
                    })
                    .collect::<Vec<_>>();
                let outputs = live_cells
                    .values()
                    .map(|cell| {
                        // use tx_index as random number

                        let lock_hash = ckb_types::packed::Script::from(cell.output.lock.clone()).calc_script_hash();
                        let tx_index = cell.tx_index.value();
                        let user = self.users.get(&lock_hash).expect("should be ok");
                        match tx_index % 3 {
                            0 => CellOutput::new_builder()
                                .capacity((cell.output.capacity.value() - 1000).pack())
                                .lock(user.single_secp256k1_lock_script_via_data())
                                .type_(self.add_tx_param.get_script_obj())
                                .build(),
                            1 => CellOutput::new_builder()
                                .capacity((cell.output.capacity.value()  - 1000).pack())
                                .lock(user.single_secp256k1_lock_script_via_type())
                                .type_(self.add_tx_param.get_script_obj())
                                .build(),
                            2 => {
                                if enabled_data1_script {
                                    CellOutput::new_builder()
                                        .capacity((cell.output.capacity.value()  - 1000).pack())
                                        .lock(user.single_secp256k1_lock_script_via_data1())
                                        .type_(self.add_tx_param.get_script_obj())
                                        .build()
                                } else {
                                    CellOutput::new_builder()
                                        .capacity((cell.output.capacity.value()  - 1000).pack())
                                        .lock(user.single_secp256k1_lock_script_via_data())
                                        .type_(self.add_tx_param.get_script_obj())
                                        .build()
                                }
                            }
                            _ => unreachable!(),
                        }
                    })
                    .collect::<Vec<_>>();
                let outputs_data = live_cells.values().map(|_| self.add_tx_param.get_output_data());
                let raw_tx = TransactionBuilder::default()
                    .inputs(inputs)
                    .outputs(outputs)
                    .outputs_data(outputs_data)
                    .cell_deps(tx_cell_deps.clone())
                    .build();
                // NOTE: We know the transaction's inputs and outputs are paired by index, so this
                // signed way is okay.
                let witnesses = live_cells.values().map(|cell| {

                    let lock_hash =  ckb_types::packed::Script::from(cell.output.lock.clone()).calc_script_hash();
                    let user = self.users.get(&lock_hash).expect("should be ok");
                    user.single_secp256k1_signed_witness(&raw_tx)
                        .as_bytes()
                        .pack()
                });
                let signed_tx = raw_tx.as_advanced_builder().witnesses(witnesses).build();
                crate::info!("signed tx:{:?}",signed_tx.to_string());
                if transaction_sender.send(TransactionView::from(signed_tx)).is_err() {
                    // SendError occurs, the corresponding transaction receiver is dead
                    return;
                }
                count += 1;
                duration_count += 1;
                if Instant::now().duration_since(start_time) >= Duration::from_secs(log_duration) {
                    let elapsed = start_time.elapsed();
                    crate::info!("[TransactionProducer] producer count: {} liveCell producer remaining :{} ,duration time:{:?}, duration tps:{} ", count,live_cell_receiver.len(),elapsed,duration_count*1000/elapsed.as_millis());
                    duration_count = 0;
                    start_time = Instant::now();
                }
            }
        }
    }
}

pub struct TransactionConsumer {
    nodes: Vec<Node>,
}


impl TransactionConsumer {
    pub fn new(nodes: Vec<Node>) -> Self {
        Self {
            nodes
        }
    }

    pub async fn run(
        self,
        transaction_receiver: Receiver<TransactionView>,
        max_concurrent_requests: usize,
        t_tx_interval: Duration,
        t_bench: Duration) {
        let start_time = Instant::now();
        let mut last_log_duration = Instant::now();
        let mut benched_transactions = 0;
        let mut duplicated_transactions = 0;
        let mut loop_count = 0;
        let mut i = 0;
        let log_duration_time = 3;

        let semaphore = Arc::new(Semaphore::new(max_concurrent_requests));
        let transactions_processed = Arc::new(AtomicUsize::new(0));
        let transactions_total_time = Arc::new(AtomicUsize::new(0));

        let mut pending_tasks = FuturesUnordered::new();

        loop {
            loop_count += 1;
            let tx = transaction_receiver
                .recv_timeout(Duration::from_secs(60 * 3))
                .expect("timeout to wait transaction_receiver");
            if t_tx_interval.as_millis() != 0 {
                async_sleep(t_tx_interval).await;
            }

            i = (i + 1) % self.nodes.len();
            let node = self.nodes[i].clone();
            let permit = semaphore.clone().acquire_owned().await;
            let tx_hash = tx.hash();
            let begin_time = Instant::now();
            let task = async move {
                let result = maybe_retry_send_transaction_async(&node, &tx).await;
                drop(permit);
                (result, tx_hash, Instant::now() - begin_time)
            };

            pending_tasks.push(tokio::spawn(task));
            while let Some(result) = pending_tasks.next().now_or_never() {
                transactions_processed.fetch_add(1, Ordering::Relaxed);

                let mut use_time = Duration::from_millis(0);

                match result {
                    Some(Ok((Ok(is_accepted), _tx_hash, cost_time))) => {
                        use_time = cost_time;
                        if is_accepted {
                            benched_transactions += 1;
                        } else {
                            duplicated_transactions += 1;
                        }
                    }
                    Some(Ok((Err(err), tx_hash, cost_time))) => {
                        use_time = cost_time;
                        // double spending, discard this transaction
                        crate::info!(
                    "consumer count :{} failed to send tx {:#x}, error: {}",
                    loop_count,
                    tx_hash,
                    err
                );
                        if !err.contains("TransactionFailedToResolve") {
                            crate::error!(
                        "failed to send tx {:#x}, error: {}",
                        tx_hash,
                        err
                    );
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("Error in task: {:?}", e);
                    }
                    None => break,
                }
                transactions_total_time.fetch_add(use_time.as_millis() as usize, Ordering::Relaxed);
            }

            if last_log_duration.elapsed() > Duration::from_secs(log_duration_time) {
                let elapsed = last_log_duration.elapsed();
                last_log_duration = Instant::now();
                let duration_count = transactions_processed.swap(0, Ordering::Relaxed);
                let duration_total_time = transactions_total_time.swap(0, Ordering::Relaxed);
                let mut duration_tps = 0;
                let mut duration_delay = 0;
                if duration_count != 0 {
                    duration_delay = duration_total_time / (duration_count as usize);
                    duration_tps = duration_count *1000 / (elapsed.as_millis() as usize);
                }
                crate::info!(
                "[TransactionConsumer] consumer :{} transactions, {} duplicated {} , transaction producer  remaining :{}, log duration {:?} ,duration send tx tps {},duration avg delay {}ms",
                loop_count,
                benched_transactions,
                duplicated_transactions,
                transaction_receiver.len(),
                elapsed,
                duration_tps,
                duration_delay
            );
            }
            if start_time.elapsed() > t_bench {
                break;
            }
        }
    }
}
