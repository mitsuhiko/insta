/// Legacy alias for `assert_yaml_snapshot`.
#[macro_export]
#[doc(hidden)]
#[deprecated(since = "0.6.0", note = "Replaced by assert_yaml_snapshot")]
macro_rules! assert_serialized_snapshot {
    ($($t:tt)*) => { $crate::assert_serialized_snapshot!($($t)*); }
}

/// Legacy alias for `assert_yaml_snapshot`.
#[macro_export]
#[deprecated(since = "0.11.0", note = "Replaced by assert_yaml_snapshot")]
macro_rules! assert_yaml_snapshot_matches {
    ($($t:tt)*) => { $crate::assert_yaml_snapshot!($($t)*); }
}

/// Legacy alias for `assert_ron_snapshot`.
#[macro_export]
#[cfg(feature = "ron")]
#[deprecated(since = "0.11.0", note = "Replaced by assert_ron_snapshot")]
macro_rules! assert_ron_snapshot_matches {
    ($($t:tt)*) => { $crate::assert_ron_snapshot!($($t)*); }
}

/// Legacy alias for `assert_ron_snapshot`.
#[macro_export]
#[cfg(not(feature = "ron"))]
#[doc(hidden)]
#[deprecated(since = "0.11.0", note = "Replaced by assert_ron_snapshot")]
macro_rules! assert_ron_snapshot_matches {
    ($($t:tt)*) => {
        compile_error!(
            "insta was compiled without ron support. Enable the ron feature to reactivate it."
        );
    };
}

/// Legacy alias for `assert_json_snapshot`.
#[macro_export]
#[deprecated(since = "0.11.0", note = "Replaced by assert_json_snapshot")]
macro_rules! assert_json_snapshot_matches {
    ($($t:tt)*) => { $crate::assert_json_snapshot!($($t)*); }
}

/// Legacy alias for `assert_debug_snapshot`.
#[macro_export]
#[deprecated(since = "0.11.0", note = "Replaced by assert_debug_snapshot")]
macro_rules! assert_debug_snapshot_matches {
    ($($t:tt)*) => { $crate::assert_debug_snapshot!($($t)*); }
}

/// Legacy alias for `assert_display_snapshot`.
#[macro_export]
#[deprecated(since = "0.11.0", note = "Replaced by assert_display_snapshot")]
macro_rules! assert_display_snapshot_matches {
    ($($t:tt)*) => { $crate::assert_display_snapshot!($($t)*); }
}

/// Legacy alias for `assert_snapshot`.
#[macro_export]
#[deprecated(since = "0.11.0", note = "Replaced by assert_snapshot")]
macro_rules! assert_snapshot_matches {
    ($($t:tt)*) => { $crate::assert_snapshot!($($t)*); }
}
