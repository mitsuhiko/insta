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
/// ```no_run
/// insta::assert_csv_snapshot!(vec![1, 2, 3]);
/// ```
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }`.  For more information
/// about redactions see [redactions](https://docs.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[cfg(feature = "csv")]
#[cfg_attr(docsrs, doc(cfg(feature = "csv")))]
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
/// ```no_run
/// insta::assert_toml_snapshot!(vec![1, 2, 3]);
/// ```
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }`.  For more information
/// about redactions refer to the [redactions feature in the guide](https://insta.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[cfg(feature = "toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
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
/// **Feature:** `yaml`
///
/// The value needs to implement the `serde::Serialize` trait and the snapshot
/// will be serialized in YAML format.  This does mean that unlike the debug
/// snapshot variant the type of the value does not appear in the output.
/// You can however use the `assert_ron_snapshot!` macro to dump out
/// the value in [RON](https://github.com/ron-rs/ron/) format which retains some
/// type information for more accurate comparisons.
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
#[cfg(feature = "yaml")]
#[cfg_attr(docsrs, doc(cfg(feature = "yaml")))]
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
/// YAML which retains some type information for more accurate comparisons.
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
#[cfg_attr(docsrs, doc(cfg(feature = "ron")))]
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
/// **Feature:** `json`
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
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
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

/// Asserts a `Serialize` snapshot in compact JSON format.
///
/// **Feature:** `json`
///
/// This works exactly like [`assert_json_snapshot!`] but serializes into a single
/// line for as long as the output is less than 120 characters.  This can be useful
/// in cases where you are working with small result outputs but comes at the cost
/// of slightly worse diffing behavior.
///
/// Example:
///
/// ```no_run
/// # use insta::*;
/// assert_compact_json_snapshot!(vec![1, 2, 3]);
/// ```
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }`.  For more information
/// about redactions refer to the [redactions feature in the guide](https://insta.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
#[macro_export]
macro_rules! assert_compact_json_snapshot {
    ($value:expr, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, JsonCompact, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}, @$snapshot:literal) => {{
        $crate::_assert_serialized_snapshot!($value, {$($k => $v),*}, JsonCompact, @$snapshot);
    }};
    ($value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, {$($k => $v),*}, JsonCompact);
    }};
    ($name:expr, $value:expr) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, JsonCompact);
    }};
    ($name:expr, $value:expr, {$($k:expr => $v:expr),*$(,)?}) => {{
        $crate::_assert_serialized_snapshot!(Some($name), $value, {$($k => $v),*}, JsonCompact);
    }};
    ($value:expr) => {{
        $crate::_assert_serialized_snapshot!($crate::_macro_support::AutoName, $value, JsonCompact);
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
/// Debug is called with `"{:#?}"`, which means this uses pretty-print.
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
    ($name:expr, $value:expr, $debug_expr:expr) => {{
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
    }};
    ($value:expr) => {
        $crate::assert_snapshot!($crate::_macro_support::AutoName, $value, stringify!($value))
    };
}

/// Settings configuration macro.
///
/// This macro lets you bind some [`Settings`](crate::Settings) temporarily.  The first argument
/// takes key value pairs that should be set, the second is the block to
/// execute.  All settings can be set (`sort_maps => value` maps to `set_sort_maps(value)`).
/// The exception are redactions which can only be set to a vector this way.
///
/// This example:
///
/// ```rust
/// insta::with_settings!({sort_maps => true}, {
///     // run snapshot test here
/// });
/// ```
///
/// Is equivalent to the following:
///
/// ```rust
/// # use insta::Settings;
/// let mut settings = Settings::clone_current();
/// settings.set_sort_maps(true);
/// settings.bind(|| {
///     // run snapshot test here
/// });
/// ```
///
/// Note: before insta 0.17 this macro used
/// [`Settings::new`](crate::Settings::new) which meant that original settings
/// were always reset rather than extended.
#[macro_export]
macro_rules! with_settings {
    ({$($k:ident => $v:expr),*$(,)?}, $body:block) => {{
        let mut settings = $crate::Settings::clone_current();
        $(
            settings._private_inner_mut().$k($v);
        )*
        settings.bind(|| $body)
    }}
}

/// Executes a closure for all input files matching a glob.
///
/// The closure is passed the path to the file.  You can use [`std::fs::read_to_string`]
/// or similar functions to load the file and process it.
///
/// ```
/// # use insta::{assert_snapshot, glob, Settings};
/// # let mut settings = Settings::clone_current();
/// # settings.set_allow_empty_glob(true);
/// # let _dropguard = settings.bind_to_scope();
/// use std::fs;
///
/// glob!("inputs/*.txt", |path| {
///     let input = fs::read_to_string(path).unwrap();
///     assert_snapshot!(input.to_uppercase());
/// });
/// ```
///
/// The `INSTA_GLOB_FILTER` environment variable can be set to only execute certain files.
/// The format of the filter is a semicolon separated filter.  For instance by setting
/// `INSTA_GLOB_FILTER` to `foo-*txt;bar-*.txt` only files starting with `foo-` or `bar-`
/// end ending in `.txt` will be executed.  When using `cargo-insta` the `--glob-filter`
/// option can be used instead.
///
/// Another effect of the globbing system is that snapshot failures within the glob macro
/// are deferred until the end of of it.  In other words this means that each snapshot
/// assertion within the `glob!` block are reported.  It can be disabled by setting
/// `INSTA_GLOB_FAIL_FAST` environment variable to `1`.
///
/// A three-argument version of this macro allows specifying a base directory
/// for the glob to start in. This allows globbing in arbitrary directories,
/// including parent directories:
///
/// ```
/// # use insta::{assert_snapshot, glob, Settings};
/// # let mut settings = Settings::clone_current();
/// # settings.set_allow_empty_glob(true);
/// # let _dropguard = settings.bind_to_scope();
/// use std::fs;
///
/// glob!("../test_data", "inputs/*.txt", |path| {
///     let input = fs::read_to_string(path).unwrap();
///     assert_snapshot!(input.to_uppercase());
/// });
/// ```
#[cfg(feature = "glob")]
#[cfg_attr(docsrs, doc(cfg(feature = "glob")))]
#[macro_export]
macro_rules! glob {
    ($base_path:expr, $glob:expr, $closure:expr) => {{
        use std::path::Path;
        let base = $crate::_macro_support::get_cargo_workspace(env!("CARGO_MANIFEST_DIR"))
            .join(Path::new(file!()).parent().unwrap())
            .join($base_path)
            .to_path_buf();

        // we try to canonicalize but on some platforms (eg: wasm) that might not work, so
        // we instead silently fall back.
        let base = base.canonicalize().unwrap_or_else(|_| base);
        $crate::_macro_support::glob_exec(env!("CARGO_MANIFEST_DIR"), &base, $glob, $closure);
    }};

    ($glob:expr, $closure:expr) => {{
        insta::glob!(".", $glob, $closure)
    }};
}

/// Utility macro to permit a multi-snapshot run where all snapshots match.
///
/// Within this block, insta will allow an assertion to be run more than once
/// (even inline) without generating another snapshot.  Instead it will assert
/// that snapshot expressions visited more than once are matching.
///
/// ```rust
/// insta::allow_duplicates! {
///     for x in (0..10).step_by(2) {
///         let is_even = x % 2 == 0;
///         insta::assert_debug_snapshot!(is_even, @"true");
///     }
/// }
/// ```
///
/// The first snapshot assertion will be used as a gold master and every further
/// assertion will be checked against it.  If they don't match the assertion will
/// fail.
#[macro_export]
macro_rules! allow_duplicates {
    ($($x:tt)*) => {
        $crate::_macro_support::with_allow_duplicates(|| {
            $($x)*
        })
    }
}
