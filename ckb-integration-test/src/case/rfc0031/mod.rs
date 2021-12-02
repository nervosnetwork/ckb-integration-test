pub(super) mod rfc0031;

const ERROR_EMPTY_EXT: &str = "Invalid: Block(EmptyBlockExtension(";
const ERROR_MAX_LIMIT: &str = "Invalid: Block(ExceededMaximumBlockExtensionBytes(";
const ERROR_UNKNOWN_FIELDS: &str = "Invalid: Block(UnknownFields(";

const RFC0031_EPOCH_NUMBER: ckb_testkit::ckb_types::core::EpochNumber = 3;
