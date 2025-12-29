//! Tests for TOML serialization in insta.
//!
//! These tests verify:
//! - Backward compatibility (single-quoted strings via Pretty)
//! - Support for types that toml 0.5.x couldn't serialize (issue #439)
//! - Proper handling of special characters, escapes, and edge cases

#![cfg(feature = "toml")]

use insta::assert_toml_snapshot;
use serde::Serialize;
use std::collections::BTreeMap;

// =============================================================================
// BACKWARD COMPATIBILITY - Critical for existing snapshots
// =============================================================================
//
// The old `toml` 0.5.x crate used single-quoted (literal) strings by default.
// The new `toml_edit` crate uses double-quoted (basic) strings by default.
//
// To maintain backward compatibility with existing snapshots, the Pretty
// visitor converts strings back to single-quoted format where possible.
//
// This is CRITICAL because changing quote style would break every existing
// TOML snapshot in downstream projects.

/// Verifies that simple strings use single quotes (backward compat with toml 0.5.x)
#[test]
fn test_toml_backward_compat_single_quotes() {
    #[derive(Serialize)]
    struct Config {
        name: String,
        version: String,
        path: String,
    }

    // CRITICAL: These MUST be single-quoted to match toml 0.5.x output
    assert_toml_snapshot!(Config {
        name: "my-package".into(),
        version: "1.0.0".into(),
        path: "/usr/local/bin".into(),
    }, @r"
    name = 'my-package'
    version = '1.0.0'
    path = '/usr/local/bin'
    ");
}

/// Verifies fallback to double quotes only when single quotes are impossible
#[test]
fn test_toml_backward_compat_quote_fallback() {
    #[derive(Serialize)]
    struct Data {
        // Can use single quotes - no special chars
        simple: String,
        // Must use double quotes - contains single quote
        with_apostrophe: String,
        // Must use multi-line - contains newline
        with_newline: String,
    }

    assert_toml_snapshot!(Data {
        simple: "hello world".into(),
        with_apostrophe: "it's here".into(),
        with_newline: "line1\nline2".into(),
    }, @r#"
    simple = 'hello world'
    with_apostrophe = '''it's here'''
    with_newline = '''
    line1
    line2'''
    "#);
}

/// Regression test for issue #439 - types that toml 0.5.x couldn't serialize
/// The old toml crate would panic with "UnsupportedType" for unit struct variants
#[test]
fn test_toml_issue_439_unit_struct_variant() {
    #[derive(Serialize)]
    #[allow(dead_code)]
    enum MyEnum {
        Variant1 {},
        Variant2 {},
    }

    #[derive(Serialize)]
    struct Config {
        value: MyEnum,
    }

    // This would PANIC with toml 0.5.x: "UnsupportedType"
    // Now it works with toml_edit
    assert_toml_snapshot!(Config { value: MyEnum::Variant1 {} }, @"[value.Variant1]");
}

// =============================================================================
// Core Types
// =============================================================================

#[test]
fn test_toml_basic_types() {
    #[derive(Serialize)]
    struct Data {
        string: String,
        integer: i64,
        unsigned: u64,
        float: f64,
        boolean: bool,
    }

    assert_toml_snapshot!(Data {
        string: "hello".into(),
        integer: -42,
        unsigned: 9007199254740991,
        float: 1.5,
        boolean: true,
    }, @r"
    string = 'hello'
    integer = -42
    unsigned = 9007199254740991
    float = 1.5
    boolean = true
    ");
}

#[test]
fn test_toml_special_floats() {
    #[derive(Serialize)]
    struct Floats {
        pos_inf: f64,
        neg_inf: f64,
        nan_value: f64,
    }

    assert_toml_snapshot!(Floats {
        pos_inf: f64::INFINITY,
        neg_inf: f64::NEG_INFINITY,
        nan_value: f64::NAN,
    }, @r"
    pos_inf = inf
    neg_inf = -inf
    nan_value = nan
    ");
}

#[test]
fn test_toml_integer_boundaries() {
    #[derive(Serialize)]
    struct Boundaries {
        min_i64: i64,
        max_i64: i64,
    }

    assert_toml_snapshot!(Boundaries {
        min_i64: i64::MIN,
        max_i64: i64::MAX,
    }, @r"
    min_i64 = -9223372036854775808
    max_i64 = 9223372036854775807
    ");
}

// =============================================================================
// String Handling - Pretty Backward Compatibility
// =============================================================================

#[test]
fn test_toml_string_quoting() {
    #[derive(Serialize)]
    struct Strings {
        simple: String,
        with_double_quotes: String,
        with_single_quotes: String,
        with_both_quotes: String,
        empty: String,
    }

    assert_toml_snapshot!(Strings {
        simple: "hello".into(),
        with_double_quotes: r#"He said "Hello""#.into(),
        with_single_quotes: "It's working".into(),
        with_both_quotes: r#"He said "It's done""#.into(),
        empty: "".into(),
    }, @r#"
    simple = 'hello'
    with_double_quotes = 'He said "Hello"'
    with_single_quotes = '''It's working'''
    with_both_quotes = '''He said "It's done"'''
    empty = ''
    "#);
}

#[test]
fn test_toml_string_escapes() {
    #[derive(Serialize)]
    struct Data {
        with_newline: String,
        with_tab: String,
        with_backslash: String,
        with_null: String,
    }

    assert_toml_snapshot!(Data {
        with_newline: "line1\nline2".into(),
        with_tab: "col1\tcol2".into(),
        with_backslash: "path\\to\\file".into(),
        with_null: "hello\0world".into(),
    }, @r#"
    with_newline = '''
    line1
    line2'''
    with_tab = 'col1	col2'
    with_backslash = 'path\to\file'
    with_null = "hello\u0000world"
    "#);
}

#[test]
fn test_toml_control_characters() {
    #[derive(Serialize)]
    struct Data {
        carriage_return: String,
        form_feed: String,
        bell: String,
    }

    assert_toml_snapshot!(Data {
        carriage_return: "line1\rline2".into(),
        form_feed: "page1\x0cpage2".into(),
        bell: "alert\x07here".into(),
    }, @r#"
    carriage_return = "line1\rline2"
    form_feed = "page1\fpage2"
    bell = "alert\u0007here"
    "#);
}

// =============================================================================
// Structures and Nesting
// =============================================================================

#[test]
fn test_toml_nested_struct() {
    #[derive(Serialize)]
    struct Inner {
        value: i32,
    }

    #[derive(Serialize)]
    struct Outer {
        name: String,
        inner: Inner,
    }

    assert_toml_snapshot!(Outer {
        name: "test".into(),
        inner: Inner { value: 42 },
    }, @r"
    name = 'test'

    [inner]
    value = 42
    ");
}

#[test]
fn test_toml_empty_struct() {
    #[derive(Serialize)]
    struct Empty {}

    #[derive(Serialize)]
    struct Container {
        empty: Empty,
    }

    assert_toml_snapshot!(Container { empty: Empty {} }, @"[empty]");
}

/// Unit structs are NOT supported by TOML - this documents the limitation
#[test]
#[should_panic(expected = "UnsupportedType")]
fn test_toml_unit_struct_unsupported() {
    #[derive(Serialize)]
    struct Marker;

    #[derive(Serialize)]
    struct Data {
        marker: Marker,
    }

    insta::_macro_support::serialize_value(
        &Data { marker: Marker },
        insta::_macro_support::SerializationFormat::Toml,
    );
}

// =============================================================================
// Arrays
// =============================================================================

#[test]
fn test_toml_arrays() {
    #[derive(Serialize)]
    struct Item {
        id: u32,
        name: String,
    }

    #[derive(Serialize)]
    struct Data {
        empty: Vec<i32>,
        numbers: Vec<i32>,
        strings: Vec<String>,
        structs: Vec<Item>,
        nested: Vec<Vec<i32>>,
    }

    assert_toml_snapshot!(Data {
        empty: vec![],
        numbers: vec![1, 2, 3],
        strings: vec!["a".into(), "b".into()],
        structs: vec![
            Item { id: 1, name: "first".into() },
            Item { id: 2, name: "second".into() },
        ],
        nested: vec![vec![1, 2], vec![3, 4]],
    }, @r"
    empty = []
    numbers = [
        1,
        2,
        3,
    ]
    strings = [
        'a',
        'b',
    ]
    nested = [
        [
        1,
        2,
    ],
        [
        3,
        4,
    ],
    ]

    [[structs]]
    id = 1
    name = 'first'

    [[structs]]
    id = 2
    name = 'second'
    ");
}

#[test]
fn test_toml_special_floats_in_array() {
    #[derive(Serialize)]
    struct Data {
        floats: Vec<f64>,
    }

    assert_toml_snapshot!(Data {
        floats: vec![1.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY],
    }, @r"
    floats = [
        1.5,
        nan,
        inf,
        -inf,
    ]
    ");
}

// =============================================================================
// Maps
// =============================================================================

#[test]
fn test_toml_maps() {
    let mut simple = BTreeMap::new();
    simple.insert("alpha", 1);
    simple.insert("beta", 2);

    let mut nested_inner = BTreeMap::new();
    nested_inner.insert("x".to_string(), 10);
    let mut nested = BTreeMap::new();
    nested.insert("coords".to_string(), nested_inner);

    #[derive(Serialize)]
    struct Data {
        simple: BTreeMap<&'static str, i32>,
        nested: BTreeMap<String, BTreeMap<String, i32>>,
    }

    assert_toml_snapshot!(Data { simple, nested }, @r"
    [simple]
    alpha = 1
    beta = 2
    [nested.coords]
    x = 10
    ");
}

#[test]
fn test_toml_integer_keys() {
    let mut map = BTreeMap::new();
    map.insert(1, "first");
    map.insert(10, "tenth");

    #[derive(Serialize)]
    struct Data {
        items: BTreeMap<i32, &'static str>,
    }

    assert_toml_snapshot!(Data { items: map }, @r"
    [items]
    1 = 'first'
    10 = 'tenth'
    ");
}

// =============================================================================
// Key Edge Cases
// =============================================================================

#[test]
fn test_toml_special_keys() {
    let mut map = BTreeMap::new();
    map.insert("", "empty key");
    map.insert("some.dotted.key", "dotted");
    map.insert("it's", "single quote");
    map.insert("key with spaces", "spaces");
    map.insert("key=value", "equals");
    map.insert("[section]", "brackets");

    #[derive(Serialize)]
    struct Data {
        items: BTreeMap<&'static str, &'static str>,
    }

    assert_toml_snapshot!(Data { items: map }, @r#"
    [items]
    "" = 'empty key'
    "[section]" = 'brackets'
    "it's" = 'single quote'
    "key with spaces" = 'spaces'
    "key=value" = 'equals'
    "some.dotted.key" = 'dotted'
    "#);
}

#[test]
fn test_toml_unicode_keys() {
    let mut map = BTreeMap::new();
    map.insert("键", "Chinese");
    map.insert("キー", "Japanese");
    map.insert("ключ", "Russian");

    #[derive(Serialize)]
    struct Data {
        items: BTreeMap<&'static str, &'static str>,
    }

    assert_toml_snapshot!(Data { items: map }, @r#"
    [items]
    "ключ" = 'Russian'
    "キー" = 'Japanese'
    "键" = 'Chinese'
    "#);
}

#[test]
fn test_toml_keyword_keys() {
    let mut map = BTreeMap::new();
    map.insert("true", "bool keyword");
    map.insert("false", "bool keyword");
    map.insert("inf", "float keyword");
    map.insert("nan", "float keyword");

    #[derive(Serialize)]
    struct Data {
        items: BTreeMap<&'static str, &'static str>,
    }

    let result = insta::_macro_support::serialize_value(
        &Data { items: map },
        insta::_macro_support::SerializationFormat::Toml,
    );
    assert!(result.contains("bool keyword"));
}

// =============================================================================
// Serde Attributes
// =============================================================================

#[test]
fn test_toml_serde_skip() {
    #[derive(Serialize)]
    #[allow(dead_code)]
    struct Data {
        included: String,
        #[serde(skip)]
        excluded: String,
    }

    assert_toml_snapshot!(Data {
        included: "visible".into(),
        excluded: "hidden".into(),
    }, @"included = 'visible'");
}

#[test]
fn test_toml_serde_rename() {
    #[derive(Serialize)]
    struct Data {
        #[serde(rename = "newName")]
        old_name: String,
    }

    assert_toml_snapshot!(Data {
        old_name: "value".into(),
    }, @"newName = 'value'");
}

#[test]
fn test_toml_serde_flatten() {
    #[derive(Serialize)]
    struct Base {
        name: String,
        age: u32,
    }

    #[derive(Serialize)]
    struct Extended {
        id: i32,
        #[serde(flatten)]
        base: Base,
    }

    assert_toml_snapshot!(Extended {
        id: 1,
        base: Base {
            name: "Alice".into(),
            age: 30,
        },
    }, @r"
    id = 1
    name = 'Alice'
    age = 30
    ");
}

#[test]
fn test_toml_option_skip_serializing_if() {
    #[derive(Serialize)]
    struct Data {
        present: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        missing: Option<i32>,
    }

    assert_toml_snapshot!(Data {
        present: Some("value".into()),
        missing: None,
    }, @"present = 'value'");
}

// =============================================================================
// Enums
// =============================================================================

#[test]
fn test_toml_enum_externally_tagged() {
    #[derive(Serialize)]
    enum Value {
        Text(String),
        Number(i32),
    }

    #[derive(Serialize)]
    struct Data {
        values: Vec<Value>,
    }

    assert_toml_snapshot!(Data {
        values: vec![Value::Text("hello".into()), Value::Number(42)],
    }, @r"
    [[values]]
    Text = 'hello'

    [[values]]
    Number = 42
    ");
}

#[test]
fn test_toml_enum_internally_tagged() {
    #[derive(Serialize)]
    #[serde(tag = "type")]
    #[allow(dead_code)]
    enum Event {
        Login { user: String },
        Logout { user: String },
    }

    #[derive(Serialize)]
    struct Data {
        event: Event,
    }

    assert_toml_snapshot!(Data {
        event: Event::Login { user: "alice".into() },
    }, @r"
    [event]
    type = 'Login'
    user = 'alice'
    ");
}

#[test]
fn test_toml_enum_adjacently_tagged() {
    #[derive(Serialize)]
    #[serde(tag = "t", content = "c")]
    #[allow(dead_code)]
    enum Message {
        Text(String),
        Number(i32),
    }

    #[derive(Serialize)]
    struct Data {
        msg: Message,
    }

    assert_toml_snapshot!(Data {
        msg: Message::Text("hello".into()),
    }, @r"
    [msg]
    t = 'Text'
    c = 'hello'
    ");
}

#[test]
fn test_toml_enum_untagged() {
    #[derive(Serialize)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum Mixed {
        Int(i32),
        Str(String),
    }

    #[derive(Serialize)]
    struct Data {
        value: Mixed,
    }

    assert_toml_snapshot!(Data {
        value: Mixed::Str("hello".into()),
    }, @"value = 'hello'");
}

// =============================================================================
// Special Types
// =============================================================================

#[test]
fn test_toml_char() {
    #[derive(Serialize)]
    struct Data {
        letter: char,
        emoji: char,
    }

    assert_toml_snapshot!(Data {
        letter: 'A',
        emoji: '🎉',
    }, @r"
    letter = 'A'
    emoji = '🎉'
    ");
}

#[test]
fn test_toml_newtype_wrapper() {
    #[derive(Serialize)]
    struct UserId(u64);

    #[derive(Serialize)]
    struct Username(String);

    #[derive(Serialize)]
    struct User {
        id: UserId,
        name: Username,
    }

    assert_toml_snapshot!(User {
        id: UserId(12345),
        name: Username("alice".into()),
    }, @r"
    id = 12345
    name = 'alice'
    ");
}

#[test]
fn test_toml_type_distinction() {
    #[derive(Serialize)]
    struct Data {
        actual_bool: bool,
        bool_string: String,
        actual_number: i32,
        number_string: String,
    }

    assert_toml_snapshot!(Data {
        actual_bool: true,
        bool_string: "true".into(),
        actual_number: 123,
        number_string: "123".into(),
    }, @r"
    actual_bool = true
    bool_string = 'true'
    actual_number = 123
    number_string = '123'
    ");
}

// =============================================================================
// Stress Tests
// =============================================================================

#[test]
fn test_toml_deep_nesting() {
    #[derive(Serialize)]
    struct L5 {
        v: i32,
    }
    #[derive(Serialize)]
    struct L4 {
        x: L5,
    }
    #[derive(Serialize)]
    struct L3 {
        x: L4,
    }
    #[derive(Serialize)]
    struct L2 {
        x: L3,
    }
    #[derive(Serialize)]
    struct L1 {
        x: L2,
    }

    let data = L1 {
        x: L2 {
            x: L3 {
                x: L4 { x: L5 { v: 42 } },
            },
        },
    };
    let result = insta::_macro_support::serialize_value(
        &data,
        insta::_macro_support::SerializationFormat::Toml,
    );
    assert!(result.contains("v = 42"));
}

#[test]
fn test_toml_large_array() {
    #[derive(Serialize)]
    struct Data {
        numbers: Vec<i32>,
    }

    let result = insta::_macro_support::serialize_value(
        &Data {
            numbers: (0..1000).collect(),
        },
        insta::_macro_support::SerializationFormat::Toml,
    );
    assert!(result.contains("999"));
}

#[test]
fn test_toml_long_string() {
    #[derive(Serialize)]
    struct Data {
        content: String,
    }

    let long = "x".repeat(10_000);
    let result = insta::_macro_support::serialize_value(
        &Data {
            content: long.clone(),
        },
        insta::_macro_support::SerializationFormat::Toml,
    );
    assert!(result.len() > 10_000);
}
