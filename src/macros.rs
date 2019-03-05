/// Alias for `assert_yaml_snapshot_matches`.
#[macro_export]
#[doc(hidden)]
#[deprecated(since = "0.6.0", note = "Replaced by assert_yaml_snapshot_matches")]
macro_rules! assert_serialized_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot_matches!($value, Yaml, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot_matches!($value, {$($k => $v),*}, Yaml, @$snapshot);
    }};
    ($name:expr, $value:expr) => {{
        $crate::_assert_serialized_snapshot_matches!($name, $value, Yaml);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::_assert_serialized_snapshot_matches!($name, $value, {$($k => $v),*}, Yaml);
    }};
}

/// Asserts a `Serialize` snapshot in YAML format.
///
/// The value needs to implement the `serde::Serialize` trait and the snapshot
/// will be serialized in YAML format.  This does mean that unlike the debug
/// snapshot variant the type of the value does not appear in the output.
/// You can however use the `assert_ron_snapshot_matches!` macro to dump out
/// the value in [RON](https://github.com/ron-rs/ron/) format which retains some
/// type information for more accurate comparisions.
///
/// Example:
///
/// ```no_run,ignore
/// assert_yaml_snapshot_matches!("snapshot_name", vec[1, 2, 3]);
/// ```
///
/// Unlike the `assert_debug_snapshot_matches` macro, this one has a secondary
/// mode where redactions can be defined.
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }`.  For more information
/// about redactions see [redactions](index.html#redactions).
///
/// Example:
///
/// ```no_run,ignore
/// assert_yaml_snapshot_matches!("name", value, {
///     ".key.to.redact" => "[replacement value]",
///     ".another.key.*.to.redact" => 42
/// });
/// ```
///
/// The replacement value can be a string, integer or any other primitive value.
///
/// For inline usage the format is `(expression, @reference_value)` where the
/// reference value must be a string literal.  If you make the initial snapshot
/// just use an empty string (`@""`).  For more information see
/// [inline snapshots](index.html#inline-snapshots).
///
/// Additionally the name is optional.  For more information see
/// [unnamed snapshots](index.html#unnamed-snapshots)
#[macro_export]
macro_rules! assert_yaml_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot_matches!($value, Yaml, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot_matches!($value, {$($k => $v),*}, Yaml, @$snapshot);
    }};
    ($name:expr, $value:expr) => {{
        $crate::_assert_serialized_snapshot_matches!(Some($name), $value, Yaml);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::_assert_serialized_snapshot_matches!(Some($name), $value, {$($k => $v),*}, Yaml);
    }};
    ($value:expr) => {{
        $crate::_assert_serialized_snapshot_matches!(None, $value, Yaml);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::_assert_serialized_snapshot_matches!(None, $value, {$($k => $v),*}, Yaml);
    }};
}

/// Asserts a `Serialize` snapshot in RON format.
///
/// This works exactly like `assert_serialized_snapshot_matches` but serializes
/// in [RON](https://github.com/ron-rs/ron/) format instead of YAML which
/// retains some type information for more accurate comparisions.
///
/// Example:
///
/// ```no_run,ignore
/// assert_ron_snapshot_matches!("snapshot_name", vec[1, 2, 3]);
/// ```
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }`.  For more information
/// about redactions see [redactions](index.html#redactions).
///
/// Additionally the name is optional.  For more information see
/// [unnamed snapshots](index.html#unnamed-snapshots)
#[macro_export]
macro_rules! assert_ron_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot_matches!($value, Ron, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot_matches!($value, {$($k => $v),*}, Ron, @$snapshot);
    }};
    ($name:expr, $value:expr) => {{
        $crate::_assert_serialized_snapshot_matches!(Some($name), $value, Ron);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::_assert_serialized_snapshot_matches!(Some($name), $value, {$($k => $v),*}, Ron);
    }};
    ($value:expr) => {{
        $crate::_assert_serialized_snapshot_matches!(None, $value, Ron);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::_assert_serialized_snapshot_matches!(None, $value, {$($k => $v),*}, Ron);
    }};
}

/// Asserts a `Serialize` snapshot in JSON format.
///
/// This works exactly like `assert_serialized_snapshot_matches` but serializes
/// in JSON format.  This is normally not recommended because it makes diffs
/// less reliable, but it can be useful for certain specialized situations.
///
/// Example:
///
/// ```no_run,ignore
/// assert_json_snapshot_matches!("snapshot_name", vec[1, 2, 3]);
/// ```
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }`.  For more information
/// about redactions see [redactions](index.html#redactions).
///
/// Additionally the name is optional.  For more information see
/// [unnamed snapshots](index.html#unnamed-snapshots)
#[macro_export]
macro_rules! assert_json_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot_matches!($value, Json, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot_matches!($value, {$($k => $v),*}, Json, @$snapshot);
    }};
    ($name:expr, $value:expr) => {{
        $crate::_assert_serialized_snapshot_matches!(Some($name), $value, Json);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::_assert_serialized_snapshot_matches!(Some($name), $value, {$($k => $v),*}, Json);
    }};
    ($value:expr) => {{
        $crate::_assert_serialized_snapshot_matches!(None, $value, Json);
    }};
    ($value:expr, {$($k:expr => $v:expr),*}) => {{
        $crate::_assert_serialized_snapshot_matches!(None, $value, {$($k => $v),*}, Json);
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! _assert_serialized_snapshot_matches {
    ($value:expr, $format:ident, @$snapshot:literal) => {{
        let value = $crate::_macro_support::serialize_value(
            &$value,
            $crate::_macro_support::SerializationFormat::$format,
            $crate::_macro_support::SnapshotLocation::Inline
        );
        $crate::assert_snapshot_matches!(
            value,
            stringify!($value),
            @$snapshot
        );
    }};
    ($value:expr, {$($k:expr => $v:expr),*}, $format:ident, @$snapshot:literal) => {{
        let vec = vec![
            $((
                $crate::_macro_support::Selector::parse($k).unwrap(),
                $crate::_macro_support::Content::from($v)
            ),)*
        ];
        let value = $crate::_macro_support::serialize_value_redacted(
            &$value,
            &vec,
            $crate::_macro_support::SerializationFormat::$format,
            $crate::_macro_support::SnapshotLocation::Inline
        );
        $crate::assert_snapshot_matches!(value, stringify!($value), @$snapshot);
    }};
    ($name:expr, $value:expr, $format:ident) => {{
        let value = $crate::_macro_support::serialize_value(
            &$value,
            $crate::_macro_support::SerializationFormat::$format,
            $crate::_macro_support::SnapshotLocation::File
        );
        $crate::assert_snapshot_matches!(
            $name,
            value,
            stringify!($value)
        );
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*}, $format:ident) => {{
        let vec = vec![
            $((
                $crate::_macro_support::Selector::parse($k).unwrap(),
                $crate::_macro_support::Content::from($v)
            ),)*
        ];
        let value = $crate::_macro_support::serialize_value_redacted(
            &$value,
            &vec,
            $crate::_macro_support::SerializationFormat::$format,
            $crate::_macro_support::SnapshotLocation::File
        );
        $crate::assert_snapshot_matches!($name, value, stringify!($value));
    }}
}

/// Asserts a `Debug` snapshot.
///
/// The value needs to implement the `fmt::Debug` trait.  This is useful for
/// simple values that do not implement the `Serialize` trait but does not
/// permit redactions.
///
/// Additionally the name is optional.  For more information see
/// [unnamed snapshots](index.html#unnamed-snapshots)
#[macro_export]
macro_rules! assert_debug_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {{
        let value = format!("{:#?}", $value);
        $crate::assert_snapshot_matches!(value, stringify!($value), @$snapshot);
    }};
    ($name:expr, $value:expr) => {{
        let value = format!("{:#?}", $value);
        $crate::assert_snapshot_matches!(Some($name), value, stringify!($value));
    }};
    ($value:expr) => {{
        let value = format!("{:#?}", $value);
        $crate::assert_snapshot_matches!(None, value, stringify!($value));
    }};
}

/// Asserts a `Display` snapshot.
///
/// The value needs to implement the `fmt::Display` trait.
///
/// Additionally the name is optional.  For more information see
/// [unnamed snapshots](index.html#unnamed-snapshots)
#[macro_export]
macro_rules! assert_display_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {{
        let value = format!("{}", $value);
        $crate::assert_snapshot_matches!(value, stringify!($value), @$snapshot);
    }};
    ($name:expr, $value:expr) => {{
        let value = format!("{}", $value);
        $crate::assert_snapshot_matches!(Some($name), value, stringify!($value));
    }};
    ($value:expr) => {{
        let value = format!("{}", $value);
        $crate::assert_snapshot_matches!(None, value, stringify!($value));
    }};
}

/// Asserts a string snapshot.
///
/// This is the most simplistic of all assertion methods.  It just accepts
/// a string to store as snapshot an does not apply any other transformations
/// on it.  This is useful to build ones own primitives.
///
/// ```no_run,ignore
/// assert_snapshot_matches!("snapshot_name", "reference value to snapshot");
/// ```
///
/// Optionally a third argument can be given as expression which will be
/// stringified as debug expression.  For more information on this look at the
/// source of this macro and other assertion macros.
///
/// Additionally the name is optional.  For more information see
/// [unnamed snapshots](index.html#unnamed-snapshots)
#[macro_export]
macro_rules! assert_snapshot_matches {
    ($value:expr, @$snapshot:literal) => {
        $crate::_assert_snapshot_matches!(
            $crate::_macro_support::ReferenceValue::Inline($snapshot),
            $value,
            stringify!($value)
        )
    };
    ($value:expr, $debug_expr:expr, @$snapshot:literal) => {
        $crate::_assert_snapshot_matches!(
            $crate::_macro_support::ReferenceValue::Inline($snapshot),
            $value,
            $debug_expr
        )
    };
    ($name:expr, $value:expr) => {
        $crate::_assert_snapshot_matches!(From::from($name), $value, stringify!($value))
    };
    ($name:expr, $value:expr, $debug_expr:expr) => {
        $crate::_assert_snapshot_matches!(From::from($name), $value, $debug_expr)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _assert_snapshot_matches {
    ($refval:expr, $value:expr, $debug_expr:expr) => {
        match &$value {
            value => {
                $crate::_macro_support::assert_snapshot(
                    $refval,
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
