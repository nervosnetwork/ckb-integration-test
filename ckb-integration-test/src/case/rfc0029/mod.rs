pub(super) mod rfc0029;

const RFC0029_EPOCH_NUMBER: ckb_testkit::ckb_types::core::EpochNumber = 3;
const RFC0029_BLOCK_NUMBER: ckb_testkit::ckb_types::core::BlockNumber = 3000;
const ERROR_MULTIPLE_MATCHES: &str = "MultipleMatches";
const ERROR_DUPLICATE_CELL_DEPS: &str = "DuplicateCellDeps";
