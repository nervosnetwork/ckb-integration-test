mod id_generator;
#[macro_use]
mod macros;
mod error;
mod v2021;

use ckb_error::AnyError;
// TODO replace json types with core types
use ckb_jsonrpc_types::{
    Transaction
};

use ckb_types::{packed::Byte32, prelude::*};
use lazy_static::lazy_static;
use std::time::{Duration, Instant};
use v2021::Inner2021;

lazy_static! {
    pub static ref HTTP_CLIENT: reqwest::blocking::Client = reqwest::blocking::Client::builder()
        .timeout(::std::time::Duration::from_secs(30))
        .build()
        .expect("reqwest Client build");
}

pub struct RpcClient {
    inner2021: Inner2021,
}

impl Clone for RpcClient {
    fn clone(&self) -> RpcClient {
        RpcClient::new(self.inner2021.url.as_str())
    }
}

impl RpcClient {
    pub fn new(uri: &str) -> Self {
        Self {
            inner2021: Inner2021::new(uri),
        }
    }

    pub fn url(&self) -> &str {
        self.inner2021.url.as_ref()
    }

    pub fn inner(&self) -> &Inner2021 {
        &self.inner2021
    }


    pub fn send_transaction(&self, tx: Transaction) -> Byte32 {
        self.send_transaction_result(tx)
            .expect("rpc call send_transaction")
    }

    pub fn send_transaction_result(&self, tx: Transaction) -> Result<Byte32, AnyError> {
            let ret = self
                .inner2021
                .send_transaction(tx, Some("passthrough".to_string()));
            // NOTE: The CKB-VM executes large-cycle transaction scripts
            // asynchronously using pause-resume approach. While the current
            // implementation has a bug: when sending a transaction embed with
            // large-cycle scripts via RPC `send_transaction` and getting a
            // successful response, the scripts may not finish executing. That
            // means that after a successful `send_transaction` request, a
            // following corresponding `get_transaction` request may get `None`.
            //
            // This if-statement is a workaround to make sure the script
            // execution finished.
            if let Ok(ref hash) = ret {
                let start_time = Instant::now();
                while start_time.elapsed() <= Duration::from_secs(20) {
                    if let Some(txstatus) = self.inner2021.get_transaction(hash.clone()).unwrap() {
                        if txstatus.tx_status.status != ckb_jsonrpc_types::Status::Unknown {
                            break;
                        }
                    }
                }
            }
            ret.map(|h256| h256.pack())
    }
}
