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
        $crate::assert_snapshot_matches!($name, value, stringify!($value));
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*}) => {{
        let mut vec = vec![];
        $(
            vec.push(($crate::Selector::parse($k).unwrap(), $crate::Value::from($v)));
        )*
        let value = $crate::_macro_support::serialize_value_redacted(&$value, &vec);
        $crate::assert_snapshot_matches!($name, value, stringify!($value));
    }}
}

/// Assets a `Debug` snapshot.
///
/// The value needs to implement the `fmt::Debug` trait.
#[macro_export]
macro_rules! assert_debug_snapshot_matches {
    ($name:expr, $value:expr) => {{
        let value = format!("{:#?}", $value);
        $crate::assert_snapshot_matches!($name, value, stringify!($value));
    }};
}

/// Assets a string snapshot.
#[macro_export]
macro_rules! assert_snapshot_matches {
    ($name:expr, $value:expr) => {
        $crate::assert_snapshot_matches!($name, $value, stringify!($value))
    };
    ($name:expr, $value:expr, $debug_expr:expr) => {
        match &$value {
            value => {
                $crate::_macro_support::assert_snapshot(
                    &$name,
                    value,
                    env!("CARGO_MANIFEST_DIR"),
                    module_path!(),
                    file!(),
                    line!(),
                    $debug_expr,
                )
                .unwrap();
            }
        }
    };
}
