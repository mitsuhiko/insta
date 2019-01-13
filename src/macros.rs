/// Assets a `Serialize` snapshot.
/// 
/// The value needs to implement the `serde::Serialize` trait.
/// 
/// This requires the `serialization` feature to be enabled.
#[cfg(feature = "serialization")]
#[macro_export]
macro_rules! assert_serialized_snapshot_matches {
    ($name:expr, $value:expr) => {{
        let value = $crate::_macro_support::serialize_value(&$value);
        $crate::assert_snapshot_matches!($name, value);
    }};
}

/// Assets a `Debug` snapshot.
/// 
/// The value needs to implement the `fmt::Debug` trait.
#[macro_export]
macro_rules! assert_debug_snapshot_matches {
    ($name:expr, $value:expr) => {{
        let value = format!("{:#?}", $value);
        $crate::assert_snapshot_matches!($name, value);
    }};
}

/// Assets a string snapshot.
#[macro_export]
macro_rules! assert_snapshot_matches {
    ($name:expr, $value:expr) => {
        match &$value {
            value => {
                $crate::_macro_support::assert_snapshot(
                    &$name,
                    value,
                    module_path!(),
                    file!(),
                    line!(),
                )
                .unwrap();
            }
        }
    };
}
