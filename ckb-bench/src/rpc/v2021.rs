use ckb_jsonrpc_types::{

  Transaction,
      TransactionWithStatusResponse
};
use ckb_types::H256;

jsonrpc!(pub struct Inner2021 {

    pub fn send_transaction(&self, tx: Transaction, outputs_validator: Option<String>) -> H256;
    pub fn get_transaction(&self, _hash: H256) -> Option<TransactionWithStatusResponse>;

});
