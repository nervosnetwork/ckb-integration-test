pub(super) mod connection;
pub(super) mod discovery;
pub(super) mod relay_transaction;
pub(super) mod v2019;

const HARDFORK_DELAY_WINDOW: u64 = 10;
const RFC0035_BLOCK_NUMBER: ckb_testkit::ckb_types::core::BlockNumber = 3000;
