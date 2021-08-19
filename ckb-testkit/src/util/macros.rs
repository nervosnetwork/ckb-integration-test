#[macro_export]
macro_rules! assert_result_eq {
    ($left:expr, $right:expr) => {
        if $left.is_err() && $right.is_err() {
            let left_raw_err = $left.as_ref().unwrap_err().to_string();
            let right_raw_err = $right.as_ref().unwrap_err().to_string();
            assert!(left_raw_err.contains(&right_raw_err) || right_raw_err.contains(&left_raw_err));
        } else {
            assert_eq!($left, $right);
        }
    };
    ($left:expr, $right:expr,) => {
        $crate::assert_result_eq!($left, $right);
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        if $left.is_err() && $right.is_err() {
            let left_raw_err = $left.as_ref().unwrap_err().to_string();
            let right_raw_err = $right.as_ref().unwrap_err().to_string();
            assert!(left_raw_err.contains(&right_raw_err) || right_raw_err.contains(&left_raw_err), $($arg)+);
        } else {
            assert_eq!($left, $right, $($arg)+);
        }
    }
}
