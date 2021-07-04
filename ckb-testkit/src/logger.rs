#[macro_export(local_inner_macros)]
macro_rules! trace {
    ($( $args:tt )*) => {
        crate::LOG_TARGET.with(|c| {
            if !c.borrow().is_empty() {
                log::trace!(target: &c.borrow(), $( $args )*);
            }
        });
    }
}

#[macro_export(local_inner_macros)]
macro_rules! debug {
    ($( $args:tt )*) => {
        crate::LOG_TARGET.with(|c| {
            if !c.borrow().is_empty() {
                log::debug!(target: &c.borrow(), $( $args )*);
            }
        });
    }
}

#[macro_export(local_inner_macros)]
macro_rules! info {
    ($( $args:tt )*) => {
        crate::LOG_TARGET.with(|c| {
            if !c.borrow().is_empty() {
                log::info!(target: &c.borrow(), $( $args )*);
            }
        });
    }
}

#[macro_export(local_inner_macros)]
macro_rules! warn {
    ($( $args:tt )*) => {
        crate::LOG_TARGET.with(|c| {
            if !c.borrow().is_empty() {
                log::warn!(target: &c.borrow(), $( $args )*);
            }
        });
    }
}

#[macro_export(local_inner_macros)]
macro_rules! error {
    ($( $args:tt )*) => {
        crate::LOG_TARGET.with(|c| {
            if !c.borrow().is_empty() {
                log::error!(target: &c.borrow(), $( $args )*);
            }
        });
    }
}
