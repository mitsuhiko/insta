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
/// This works exactly like [`crate::assert_yaml_snapshot!`]
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
/// It's in the form `{ selector => replacement }` or `match .. { selector => replacement }`.
/// For more information about redactions refer to the [redactions feature in
/// the guide](https://insta.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[cfg(feature = "csv")]
#[cfg_attr(docsrs, doc(cfg(feature = "csv")))]
#[macro_export]
macro_rules! assert_csv_snapshot {
    ($($arg:tt)*) => {
        $crate::_assert_serialized_snapshot!(format=Csv, $($arg)*);
    };
}

/// Asserts a `Serialize` snapshot in TOML format.
///
/// **Feature:** `toml` (disabled by default)
///
/// This works exactly like [`crate::assert_yaml_snapshot!`]
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
/// It's in the form `{ selector => replacement }` or `match .. { selector => replacement }`.
/// For more information about redactions refer to the [redactions feature in
/// the guide](https://insta.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[cfg(feature = "toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
#[macro_export]
macro_rules! assert_toml_snapshot {
    ($($arg:tt)*) => {
        $crate::_assert_serialized_snapshot!(format=Toml, $($arg)*);
    };
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
/// Unlike the [`crate::assert_debug_snapshot!`]
/// macro, this one has a secondary mode where redactions can be defined.
///
/// The third argument to the macro can be an object expression for redaction.
/// It's in the form `{ selector => replacement }` or `match .. { selector => replacement }`.
/// For more information about redactions refer to the [redactions feature in
/// the guide](https://insta.rs/docs/redactions/).
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
    ($($arg:tt)*) => {
        $crate::_assert_serialized_snapshot!(format=Yaml, $($arg)*);
    };
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
/// It's in the form `{ selector => replacement }` or `match .. { selector => replacement }`.
/// For more information about redactions refer to the [redactions feature in
/// the guide](https://insta.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[cfg(feature = "ron")]
#[cfg_attr(docsrs, doc(cfg(feature = "ron")))]
#[macro_export]
macro_rules! assert_ron_snapshot {
    ($($arg:tt)*) => {
        $crate::_assert_serialized_snapshot!(format=Ron, $($arg)*);
    };
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
/// It's in the form `{ selector => replacement }` or `match .. { selector => replacement }`.
/// For more information about redactions refer to the [redactions feature in
/// the guide](https://insta.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
#[macro_export]
macro_rules! assert_json_snapshot {
    ($($arg:tt)*) => {
        $crate::_assert_serialized_snapshot!(format=Json, $($arg)*);
    };
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
/// It's in the form `{ selector => replacement }` or `match .. { selector => replacement }`.
/// For more information about redactions refer to the [redactions feature in
/// the guide](https://insta.rs/docs/redactions/).
///
/// The snapshot name is optional but can be provided as first argument.
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
#[macro_export]
macro_rules! assert_compact_json_snapshot {
    ($($arg:tt)*) => {
        $crate::_assert_serialized_snapshot!(format=JsonCompact, $($arg)*);
    };
}

// This macro handles optional trailing commas.
#[doc(hidden)]
#[macro_export]
macro_rules! _assert_serialized_snapshot {
    // If there are redaction expressions and an inline snapshot, capture
    // the redactions expressions and pass to `_assert_snapshot_base`
    //
    // Note that if we could unify the Inline & File representations of snapshots
    // redactions we could unify some of these branches.
    (format=$format:ident, $value:expr, $(match ..)? {$($k:expr => $v:expr),* $(,)?}, @$snapshot:literal $(,)?) => {{
        let transform = |value| {
            let (_, value) = $crate::_prepare_snapshot_for_redaction!(value, {$($k => $v),*}, $format, Inline);
            value
        };
        $crate::_assert_snapshot_base!(transform=transform, $value, @$snapshot);
    }};
    // If there are redaction expressions and no name, add a auto-generated name, call self
    (format=$format:ident, $value:expr, $(match ..)? {$($k:expr => $v:expr),* $(,)?} $(,)?) => {{
        $crate::_assert_serialized_snapshot!(format=$format, $crate::_macro_support::AutoName, $value, {$($k => $v),*});
    }};
    // If there are redaction expressions, capture and pass to `_assert_snapshot_base`
    (format=$format:ident, $name:expr, $value:expr, $(match ..)? {$($k:expr => $v:expr),* $(,)?} $(,)?) => {{
        let transform = |value| {
            let (_, value) = $crate::_prepare_snapshot_for_redaction!(value, {$($k => $v),*}, $format, File);
            value
        };
        $crate::_assert_snapshot_base!(transform=transform, $name, $value);
    }};
    // If there's an inline snapshot, capture serialization function and pass to
    // `_assert_snapshot_base`, specifying `Inline`
    (format=$format:ident, $($arg:expr),*, @$snapshot:literal $(,)?) => {{
        let transform = |value| {$crate::_macro_support::serialize_value(
            &value,
            $crate::_macro_support::SerializationFormat::$format,
            $crate::_macro_support::SnapshotLocation::Inline
        )};
        $crate::_assert_snapshot_base!(transform = transform, $($arg),*, @$snapshot);
    }};
    // Capture serialization function and pass to `_assert_snapshot_base`,
    // specifying `File`
    (format=$format:ident, $($arg:expr),* $(,)?) => {{
        let transform = |value| {$crate::_macro_support::serialize_value(
            &value,
            $crate::_macro_support::SerializationFormat::$format,
            $crate::_macro_support::SnapshotLocation::File
        )};
        $crate::_assert_snapshot_base!(transform = transform, $($arg),*);
    }};
}

#[cfg(feature = "redactions")]
#[doc(hidden)]
#[macro_export]
macro_rules! _prepare_snapshot_for_redaction {
    ($value:expr, {$($k:expr => $v:expr),*}, $format:ident, $location:ident) => {
        {
            let vec = std::vec![
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
    ($value:expr, {$($k:expr => $v:expr),*}, $format:ident, $location:ident) => {
        compile_error!("insta was compiled without redaction support.");
    };
}

/// Asserts a `Debug` snapshot.
///
/// The value needs to implement the `fmt::Debug` trait.  This is useful for
/// simple values that do not implement the `Serialize` trait, but does not
/// permit redactions.
///
/// Debug is called with `"{:#?}"`, which means this uses pretty-print.
#[macro_export]
macro_rules! assert_debug_snapshot {
    ($($arg:tt)*) => {
        $crate::_assert_snapshot_base!(transform=|v| std::format!("{:#?}", v), $($arg)*)
    };
}

// A helper macro which takes a closure as `transform`, and runs the closure on
// the value. This allows us to implement other macros with a small wrapper. All
// snapshot macros eventually call this macro.
//
// This macro handles optional trailing commas.
#[doc(hidden)]
#[macro_export]
macro_rules! _assert_snapshot_base {
    // If there's an inline literal value, wrap the literal in a
    // `ReferenceValue::Inline`, call self.
    (transform=$transform:expr, $($arg:expr),*, @$snapshot:literal $(,)?) => {
        $crate::_assert_snapshot_base!(
            transform = $transform,
            #[allow(clippy::needless_raw_string_hashes)]
            $crate::_macro_support::ReferenceValue::Inline($snapshot),
            $($arg),*
        )
    };
    // If there's no debug_expr, use the stringified value, call self.
    (transform=$transform:expr, $name:expr, $value:expr $(,)?) => {
        $crate::_assert_snapshot_base!(transform = $transform, $name, $value, stringify!($value))
    };
    // If there's no name (and necessarily no debug expr), auto generate the
    // name, call self.
    (transform=$transform:expr, $value:expr $(,)?) => {
        $crate::_assert_snapshot_base!(
            transform = $transform,
            $crate::_macro_support::AutoName,
            $value
        )
    };
    // The main macro body — every call to this macro should end up here.
    (transform=$transform:expr, $name:expr, $value:expr, $debug_expr:expr $(,)?) => {
        $crate::_macro_support::assert_snapshot(
            $name.into(),
            #[allow(clippy::redundant_closure_call)]
            &$transform(&$value),
            env!("CARGO_MANIFEST_DIR"),
            $crate::_function_name!(),
            module_path!(),
            file!(),
            line!(),
            $debug_expr,
        )
        .unwrap()
    };
}

/// Asserts a `Display` snapshot.
///
/// This is now deprecated, replaced by the more generic `assert_snapshot!()`
#[macro_export]
#[deprecated = "use assert_snapshot!() instead"]
macro_rules! assert_display_snapshot {
    ($($arg:tt)*) => {
        $crate::assert_snapshot!($($arg)*)
    };
}

/// Asserts a string snapshot.
///
/// This is the simplest of all assertion methods.  It accepts any value that
/// implements `fmt::Display`.
///
/// ```no_run
/// # use insta::*;
/// // implicitly named
/// assert_snapshot!("reference value to snapshot");
/// // named
/// assert_snapshot!("snapshot_name", "reference value to snapshot");
/// // inline
/// assert_snapshot!("reference value", @"reference value");
/// ```
///
/// Optionally a third argument can be given as an expression to be stringified
/// as the debug expression.  For more information on this, check out
/// <https://insta.rs/docs/snapshot-types/>.
#[macro_export]
macro_rules! assert_snapshot {
    ($($arg:tt)*) => {
        $crate::_assert_snapshot_base!(transform=|v| std::format!("{}", v), $($arg)*)
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
