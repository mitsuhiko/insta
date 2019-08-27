/// Legacy alias for `assert_yaml_snapshot`.
#[macro_export]
#[deprecated(since = "0.6.0", note = "Replaced by assert_yaml_snapshot")]
macro_rules! assert_serialized_snapshot {
    ($value:expr, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, Yaml, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, {$($k => $v),*}, Yaml, @$snapshot);
    }};
    ($name:expr, $value:expr) => {{
        $crate::_assert_serialized_snapshot!($name, $value, Yaml);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::_assert_serialized_snapshot!($name, $value, {$($k => $v),*}, Yaml);
    }};
}

/// Legacy alias for `assert_yaml_snapshot`.
#[macro_export]
#[deprecated(since = "0.11.0", note = "Replaced by assert_yaml_snapshot")]
macro_rules! assert_yaml_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {{
        $crate::assert_yaml_snapshot($value, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}, @$snapshot:literal) => {{
        $crate::assert_yaml_snapshot!($value, {$($k => $v),*}, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::assert_yaml_snapshot!($value, {$($k => $v),*});
    }};
    ($name:expr, $value:expr) => {{
        $crate::assert_yaml_snapshot!($name, $value);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::assert_yaml_snapshot!($name, $value, {$($k => $v),*});
    }};
    ($value:expr) => {{
        $crate::assert_yaml_snapshot!($value);
    }};
}

/// Legacy alias for `assert_ron_snapshot`.
#[macro_export]
#[cfg(feature = "ron")]
#[deprecated(since = "0.11.0", note = "Replaced by assert_ron_snapshot")]
macro_rules! assert_ron_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {{
        $crate::assert_ron_snapshot($value, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}, @$snapshot:literal) => {{
        $crate::assert_ron_snapshot!($value, {$($k => $v),*}, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::assert_ron_snapshot!($value, {$($k => $v),*});
    }};
    ($name:expr, $value:expr) => {{
        $crate::assert_ron_snapshot!($name, $value);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::assert_ron_snapshot!($name, $value, {$($k => $v),*});
    }};
    ($value:expr) => {{
        $crate::assert_ron_snapshot!($value);
    }};
}

/// Legacy alias for `assert_json_snapshot`.
#[macro_export]
#[deprecated(since = "0.11.0", note = "Replaced by assert_json_snapshot")]
macro_rules! assert_json_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {{
        $crate::assert_json_snapshot($value, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}, @$snapshot:literal) => {{
        $crate::assert_json_snapshot!($value, {$($k => $v),*}, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::assert_json_snapshot!($value, {$($k => $v),*});
    }};
    ($name:expr, $value:expr) => {{
        $crate::assert_json_snapshot!($name, $value);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::assert_json_snapshot!($name, $value, {$($k => $v),*});
    }};
    ($value:expr) => {{
        $crate::assert_json_snapshot!($value);
    }};
}

/// Legacy alias for `assert_debug_snapshot`.
#[macro_export]
#[deprecated(since = "0.11.0", note = "Replaced by assert_debug_snapshot")]
macro_rules! assert_debug_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {{
        $crate::assert_debug_snapshot!($value, @$snapshot);
    }};
    ($name:expr, $value:expr) => {{
        $crate::assert_debug_snapshot!($name, $value);
    }};
    ($value:expr) => {{
        $crate::assert_debug_snapshot!($value);
    }};
}

/// Legacy alias for `assert_display_snapshot`.
#[macro_export]
#[deprecated(since = "0.11.0", note = "Replaced by assert_display_snapshot")]
macro_rules! assert_display_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {{
        $crate::assert_display_snapshot!($value, @$snapshot);
    }};
    ($name:expr, $value:expr) => {{
        $crate::assert_display_snapshot!($name, $value);
    }};
    ($value:expr) => {{
        $crate::assert_display_snapshot!($value);
    }};
}

/// Legacy alias for `assert_snapshot`.
#[macro_export]
#[deprecated(since = "0.11.0", note = "Replaced by assert_snapshot")]
macro_rules! assert_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {
        $crate::assert_snapshot!($value, @$snapshot);
    };
    ($value:expr, $debug_expr:expr, @$snapshot:literal) => {
        $crate::assert_snapshot!($value, $debug_expr, @$snapshot);
    };
    ($name:expr, $value:expr) => {
        $crate::assert_snapshot!($name, $value);
    };
    ($name:expr, $value:expr, $debug_expr:expr) => {
        $crate::assert_snapshot!($name, $value, $debug_expr);
    };
    ($value:expr) => {
        $crate::assert_snapshot!($value);
    };
}
