pub(super) mod after_switch;
pub(super) mod before_switch;
mod util;

const ERROR_EMPTY_EXT: &str = "Invalid: Block(EmptyBlockExtension(";
const ERROR_MAX_LIMIT: &str = "Invalid: Block(ExceededMaximumBlockExtensionBytes(";
const ERROR_INVALID_PARAMS: &str = "Invalid params: unknown field `extension`";
const ERROR_UNKNOWN_FIELDS: &str = "Invalid: Block(UnknownFields(";
