pub mod logger;
pub mod node;
pub mod nodes;
pub mod rpc;
pub mod util;

use std::cell::RefCell;

thread_local! {
    // Initialize at beginning of running case
    pub static LOG_TARGET: RefCell<String> = RefCell::new(String::new());
}
