pub(super) mod rfc0034;

const RFC0034_EPOCH_NUMBER: ckb_testkit::ckb_types::core::EpochNumber = 3;
const ERROR_INVALID_ECALL: &str = "InvalidEcall";
#[allow(dead_code)]
const ERROR_INVALID_VM_VERSION: &str = " Invalid VM Version";
#[allow(dead_code)]
const ERROR_OUT_OF_BOUND: &str = "error code 1 in the page";
