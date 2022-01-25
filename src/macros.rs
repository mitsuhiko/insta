/// Utility macro to return the name of the current function.
#[doc(hidden)]
#[macro_export]
macro_rules! _function_name {
    () => {{
        fn f() {}
        fn type_name_of_val<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let mut name = type_name_of_val(f).strip_suffix("::f").unwrap_or("");
        while let Some(rest) = name.strip_suffix("::{{closure}}") {
            name = rest;
        }
        name
    }};
}

/// Asserts a `Serialize` snapshot in CSV format.
///
/// **Feature:** `csv` (disabled by default)
///
/// This works exactly like [`assert_yaml_snapshot!`]
/// but serializes in [CSV](https://github.com/burntsushi/rust-csv) format instead of
/// YAML.
///
/// Example:
///
/// ```no_run,ignore
/// assert_csv_snapshot!(vec[1, 2, 3]);
/// ```
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }`.  For more information
/// about redactions see [redactions](https://docs.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[cfg(feature = "csv")]
#[macro_export]
macro_rules! assert_csv_snapshot {
    ($value:expr, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, Csv, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, {$($k => $v),*}, Csv, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, {$($k => $v),*}, Csv);
    }};
    ($name:expr, $value:expr) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, Csv);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, {$($k => $v),*}, Csv);
    }};
    ($value:expr) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, Csv);
    }};
}

/// Asserts a `Serialize` snapshot in TOML format.
///
/// **Feature:** `toml` (disabled by default)
///
/// This works exactly like [`assert_yaml_snapshot!`]
/// but serializes in [TOML](https://github.com/alexcrichton/toml-rs) format instead of
/// YAML.  Note that TOML cannot represent all values due to limitations in the
/// format.
///
/// Example:
///
/// ```no_run,ignore
/// assert_toml_snapshot!(vec[1, 2, 3]);
/// ```
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }`.  For more information
/// about redactions refer to the [redactions feature in the guide](https://insta.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[cfg(feature = "toml")]
#[macro_export]
macro_rules! assert_toml_snapshot {
    ($value:expr, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, Toml, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, {$($k => $v),*}, Toml, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, {$($k => $v),*}, Toml);
    }};
    ($name:expr, $value:expr) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, Toml);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, {$($k => $v),*}, Toml);
    }};
    ($value:expr) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, Toml);
    }};
}

/// Asserts a `Serialize` snapshot in YAML format.
///
/// The value needs to implement the `serde::Serialize` trait and the snapshot
/// will be serialized in YAML format.  This does mean that unlike the debug
/// snapshot variant the type of the value does not appear in the output.
/// You can however use the `assert_ron_snapshot!` macro to dump out
/// the value in [RON](https://github.com/ron-rs/ron/) format which retains some
/// type information for more accurate comparisions.
///
/// Example:
///
/// ```no_run
/// # use insta::*;
/// assert_yaml_snapshot!(vec![1, 2, 3]);
/// ```
///
/// Unlike the [`assert_debug_snapshot!`]
/// macro, this one has a secondary mode where redactions can be defined.
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }`.  For more information
/// about redactions refer to the [redactions feature in the guide](https://insta.rs/docs/redactions/).
///
/// Example:
///
#[cfg_attr(feature = "redactions", doc = " ```no_run")]
#[cfg_attr(not(feature = "redactions"), doc = " ```ignore")]
/// # use insta::*; use serde::Serialize;
/// # #[derive(Serialize)] struct Value; let value = Value;
/// assert_yaml_snapshot!(value, {
///     ".key.to.redact" => "[replacement value]",
///     ".another.key.*.to.redact" => 42
/// });
/// ```
///
/// The replacement value can be a string, integer or any other primitive value.
///
/// For inline usage the format is `(expression, @reference_value)` where the
/// reference value must be a string literal.  If you make the initial snapshot
/// just use an empty string (`@""`).
///
/// The snapshot name is optional but can be provided as first argument.
#[macro_export]
macro_rules! assert_yaml_snapshot {
    ($value:expr, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, Yaml, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, {$($k => $v),*}, Yaml, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, {$($k => $v),*}, Yaml);
    }};
    ($name:expr, $value:expr) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, Yaml);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, {$($k => $v),*}, Yaml);
    }};
    ($value:expr) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, Yaml);
    }};
}

/// Asserts a `Serialize` snapshot in RON format.
///
/// **Feature:** `ron` (disabled by default)
///
/// This works exactly like [`assert_yaml_snapshot!`]
/// but serializes in [RON](https://github.com/ron-rs/ron/) format instead of
/// YAML which retains some type information for more accurate comparisions.
///
/// Example:
///
/// ```no_run
/// # use insta::*;
/// assert_ron_snapshot!(vec![1, 2, 3]);
/// ```
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }`.  For more information
/// about redactions refer to the [redactions feature in the guide](https://insta.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[cfg(feature = "ron")]
#[macro_export]
macro_rules! assert_ron_snapshot {
    ($value:expr, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, Ron, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, {$($k => $v),*}, Ron, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, {$($k => $v),*}, Ron);
    }};
    ($name:expr, $value:expr) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, Ron);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, {$($k => $v),*}, Ron);
    }};
    ($value:expr) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, Ron);
    }};
}

/// Asserts a `Serialize` snapshot in JSON format.
///
/// This works exactly like [`assert_yaml_snapshot!`] but serializes in JSON format.
/// This is normally not recommended because it makes diffs less reliable, but it can
/// be useful for certain specialized situations.
///
/// Example:
///
/// ```no_run
/// # use insta::*;
/// assert_json_snapshot!(vec![1, 2, 3]);
/// ```
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }`.  For more information
/// about redactions refer to the [redactions feature in the guide](https://insta.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[macro_export]
macro_rules! assert_json_snapshot {
    ($value:expr, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, Json, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, {$($k => $v),*}, Json, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, {$($k => $v),*}, Json);
    }};
    ($name:expr, $value:expr) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, Json);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, {$($k => $v),*}, Json);
    }};
    ($value:expr) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, Json);
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! _assert_serialized_snapshot {
    ($value:expr, $format:ident, @$snapshot:literal) => {{
        let value = $crate::_macro_support::serialize_value(
            &$value,
            $crate::_macro_support::SerializationFormat::$format,
            $crate::_macro_support::SnapshotLocation::Inline
        );
        $crate::assert_snapshot!(
            value,
            stringify!($value),
            @$snapshot
        );
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}, $format:ident, @$snapshot:literal) => {{
        let (vec, value) = $crate::_prepare_snapshot_for_redaction!($value, {$($k => $v),*}, $format, Inline);
        $crate::assert_snapshot!(value, stringify!($value), @$snapshot);
    }};
    ($name:expr, $value:expr, $format:ident) => {{
        let value = $crate::_macro_support::serialize_value(
            &$value,
            $crate::_macro_support::SerializationFormat::$format,
            $crate::_macro_support::SnapshotLocation::File
        );
        $crate::assert_snapshot!(
            $name,
            value,
            stringify!($value)
        );
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*$(,)?}, $format:ident) => {{
        let (vec, value) = $crate::_prepare_snapshot_for_redaction!($value, {$($k => $v),*}, $format, File);
        $crate::assert_snapshot!($name, value, stringify!($value));
    }}
}

#[cfg(feature = "redactions")]
#[doc(hidden)]
#[macro_export]
macro_rules! _prepare_snapshot_for_redaction {
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}, $format:ident, $location:ident) => {
        {
            let vec = vec![
                $((
                    $crate::_macro_support::Selector::parse($k).unwrap(),
                    $crate::_macro_support::Redaction::from($v)
                ),)*
            ];
            let value = $crate::_macro_support::serialize_value_redacted(
                &$value,
                &vec,
                $crate::_macro_support::SerializationFormat::$format,
                $crate::_macro_support::SnapshotLocation::$location
            );
            (vec, value)
        }
    }
}

#[cfg(not(feature = "redactions"))]
#[doc(hidden)]
#[macro_export]
macro_rules! _prepare_snapshot_for_redaction {
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}, $format:ident, $location:ident) => {
        compile_error!("insta was compiled without redaction support.");
    };
}

/// Asserts a `Debug` snapshot.
///
/// The value needs to implement the `fmt::Debug` trait.  This is useful for
/// simple values that do not implement the `Serialize` trait but does not
/// permit redactions.
///
/// The snapshot name is optional.
#[macro_export]
macro_rules! assert_debug_snapshot {
    ($value:expr, @$snapshot:literal) => {{
        let value = format!("{:#?}", $value);
        $crate::assert_snapshot!(value, stringify!($value), @$snapshot);
    }};
    ($name:expr, $value:expr) => {{
        let value = format!("{:#?}", $value);
        $crate::assert_snapshot!(Some($name), value, stringify!($value));
    }};
    ($value:expr) => {{
        let value = format!("{:#?}", $value);
        $crate::assert_snapshot!($crate::_macro_support::AutoName, value, stringify!($value));
    }};
}

/// Asserts a `Display` snapshot.
///
/// The value needs to implement the `fmt::Display` trait.
///
/// The snapshot name is optional.
#[macro_export]
macro_rules! assert_display_snapshot {
    ($value:expr, @$snapshot:literal) => {{
        let value = format!("{}", $value);
        $crate::assert_snapshot!(value, stringify!($value), @$snapshot);
    }};
    ($name:expr, $value:expr) => {{
        let value = format!("{}", $value);
        $crate::assert_snapshot!(Some($name), value, stringify!($value));
    }};
    ($value:expr) => {{
        let value = format!("{}", $value);
        $crate::assert_snapshot!($crate::_macro_support::AutoName, value, stringify!($value));
    }};
}

/// Asserts a string snapshot.
///
/// This is the most simplistic of all assertion methods.  It just accepts
/// a string to store as snapshot an does not apply any other transformations
/// on it.  This is useful to build ones own primitives.
///
/// ```no_run
/// # use insta::*;
/// assert_snapshot!("reference value to snapshot");
/// ```
///
/// Optionally a third argument can be given as expression which will be
/// stringified as debug expression.  For more information on this look at the
/// source of this macro and other assertion macros.
///
/// The snapshot name is optional.
#[macro_export]
macro_rules! assert_snapshot {
    ($value:expr, @$snapshot:literal) => {
        $crate::assert_snapshot!(
            $crate::_macro_support::ReferenceValue::Inline($snapshot),
            $value,
            stringify!($value)
        )
    };
    ($value:expr, $debug_expr:expr, @$snapshot:literal) => {
        $crate::assert_snapshot!(
            $crate::_macro_support::ReferenceValue::Inline($snapshot),
            $value,
            $debug_expr
        )
    };
    ($name:expr, $value:expr) => {
        $crate::assert_snapshot!($name, $value, stringify!($value))
    };
    ($name:expr, $value:expr, $debug_expr:expr) => {
        $crate::_macro_support::assert_snapshot(
            // Creates a ReferenceValue::Named variant
            $name.into(),
            &$value,
            env!("CARGO_MANIFEST_DIR"),
            $crate::_function_name!(),
            module_path!(),
            file!(),
            line!(),
            $debug_expr,
        )
        .unwrap()
    };
    ($value:expr) => {
        $crate::assert_snapshot!($crate::_macro_support::AutoName, $value, stringify!($value))
    };
}

/// Settings configuration macro.
///
/// This macro lets you bind some settings temporarily.  The first argument
/// takes key value pairs that should be set, the second is the block to
/// execute.  All settings can be set (`sort_maps => value` maps roughly
/// to `set_sort_maps(value)`).
///
/// ```rust
/// insta::with_settings!({sort_maps => true}, {
///     // run snapshot test here
/// });
/// ```
#[macro_export]
macro_rules! with_settings {
    ({$($k:ident => $v:expr),*$(,)?}, $body:block) => {{
        let mut settings = $crate::Settings::new();
        $(
            settings._private_inner_mut().$k = $v.into();
        )*
        settings.bind(|| $body)
    }}
}

/// Executes a closure for all input files matching a glob.
///
/// The closure is passed the path to the file.
#[cfg(feature = "glob")]
#[macro_export]
macro_rules! glob {
    ($glob:expr, $closure:expr) => {{
        let base = $crate::_macro_support::get_cargo_workspace(env!("CARGO_MANIFEST_DIR"))
            .join(file!())
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap_or_else(|e| panic!("failed to canonicalize insta::glob! base path: {}", e));
        $crate::_macro_support::glob_exec(&base, $glob, $closure);
    }};
}
