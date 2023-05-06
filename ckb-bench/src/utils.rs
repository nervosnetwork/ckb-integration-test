use ckb_testkit::ckb_types::core::TransactionView;
use ckb_testkit::Node;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tokio::time::sleep as async_sleep;
use futures::future::poll_fn;

pub fn maybe_retry_send_transaction(node: &Node, tx: &TransactionView) -> Result<bool, String> {
    let mut last_logging_time = Instant::now();
    loop {
        let result = node.rpc_client().send_transaction_result(tx.data().into());
        match result {
            Ok(_hash) => return Ok(true),
            Err(err) => {
                let raw_err = err.to_string();
                if raw_err.contains("PoolIsFull") {
                    sleep(Duration::from_millis(10));
                    if last_logging_time.elapsed() >= Duration::from_secs(5) {
                        last_logging_time = Instant::now();
                        ckb_testkit::debug!(
                            "retry to send tx {:#x} as the pool is full",
                            tx.hash()
                        );
                    }
                } else if raw_err.contains("PoolRejectedDuplicatedTransaction") {
                    return Ok(false);
                } else {
                    return Err(raw_err);
                }
            }
        }
    }
}

pub async fn maybe_retry_send_transaction_async(node: &Node, tx: &TransactionView) -> Result<bool, String> {
    let mut last_logging_time = Instant::now();
    loop {
        // let mut begin = Instant::now();
        let result = poll_fn(|_| {
            std::task::Poll::Ready(node.rpc_client().send_transaction_result(tx.data().into()))
        }).await;
        // ckb_testkit::debug!("tx delay:{:?},rt:{:?}",Instant::now() - begin,result);
        match result {
            Ok(_hash) => return Ok(true),
            Err(err) => {
                let raw_err = err.to_string();
                ckb_testkit::info!("error:{}",raw_err);
                if raw_err.contains("PoolIsFull") {
                    async_sleep(Duration::from_millis(10)).await;
                    if last_logging_time.elapsed() >= Duration::from_secs(5) {
                        last_logging_time = Instant::now();
                        ckb_testkit::debug!(
                            "retry to send tx {:#x} as the pool is full",
                            tx.hash()
                        );
                    }
                } else if raw_err.contains("PoolRejectedDuplicatedTransaction") {
                    return Ok(false);
                } else {
                    return Err(raw_err);
                }
            }
        }
    }
}
