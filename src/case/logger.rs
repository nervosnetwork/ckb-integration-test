#[macro_export(local_inner_macros)]
macro_rules! trace {
    ($( $args:tt )*) => {
        crate::case::CASE_NAME.with(|c| {
            log::trace!(target: &c.borrow(), $( $args )*);
        });
    }
}

#[macro_export(local_inner_macros)]
macro_rules! debug {
    ($( $args:tt )*) => {
        crate::case::CASE_NAME.with(|c| {
            log::debug!(target: &c.borrow(), $( $args )*);
        });
    }
}

#[macro_export(local_inner_macros)]
macro_rules! info {
    ($( $args:tt )*) => {
        crate::case::CASE_NAME.with(|c| {
            log::info!(target: &c.borrow(), $( $args )*);
        });
    }
}

#[macro_export(local_inner_macros)]
macro_rules! warn {
    ($( $args:tt )*) => {
        crate::case::CASE_NAME.with(|c| {
            log::warn!(target: &c.borrow(), $( $args )*);
        });
    }
}

#[macro_export(local_inner_macros)]
macro_rules! error {
    ($( $args:tt )*) => {
        crate::case::CASE_NAME.with(|c| {
            log::error!(target: &c.borrow(), $( $args )*);
        });
    }
}
