use std::fmt::{Display, Write};

use crate::content::Content;

/// The maximum number of characters a line may have before a container is
/// broken up when [`to_string_compact`] is used.
const COMPACT_MAX_CHARS: usize = 120;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Format {
    Condensed,
    SingleLine,
    Pretty,
    /// Like [`Format::Pretty`], but any container that fits on the current
    /// line within [`COMPACT_MAX_CHARS`] is written in [`Format::SingleLine`]
    /// style instead of being expanded.
    PrettyCompact,
}

/// Serializes a serializable to JSON.
pub struct Serializer {
    out: String,
    format: Format,
    indentation: usize,
}

impl Serializer {
    /// Creates a new [`Serializer`] that writes into the given writer.
    pub fn new() -> Serializer {
        Serializer {
            out: String::new(),
            format: Format::Condensed,
            indentation: 0,
        }
    }

    pub fn into_result(self) -> String {
        self.out
    }

    fn write_indentation(&mut self) {
        if self.is_pretty() {
            write!(self.out, "{: ^1$}", "", self.indentation * 2).unwrap();
        }
    }

    fn start_container(&mut self, c: char) {
        self.write_char(c);
        self.indentation += 1;
    }

    fn end_container(&mut self, c: char, empty: bool) {
        self.indentation -= 1;
        if self.is_pretty() && !empty {
            self.write_char('\n');
            self.write_indentation();
        }
        self.write_char(c);
    }

    fn write_comma(&mut self, first: bool) {
        match self.format {
            Format::Pretty | Format::PrettyCompact => {
                if first {
                    self.write_char('\n');
                } else {
                    self.write_str(",\n");
                }
                self.write_indentation();
            }
            Format::Condensed => {
                if !first {
                    self.write_char(',');
                }
            }
            Format::SingleLine => {
                if !first {
                    self.write_str(", ");
                }
            }
        }
    }

    fn write_colon(&mut self) {
        match self.format {
            Format::Pretty | Format::PrettyCompact | Format::SingleLine => self.write_str(": "),
            Format::Condensed => self.write_char(':'),
        }
    }

    fn serialize_array(&mut self, items: &[Content]) {
        self.start_container('[');
        for (idx, item) in items.iter().enumerate() {
            self.write_comma(idx == 0);
            self.serialize_reserved(item, trailing_comma(idx, items.len()));
        }
        self.end_container(']', items.is_empty());
    }

    fn serialize_object(&mut self, fields: &[(&str, Content)]) {
        self.start_container('{');
        for (idx, (key, value)) in fields.iter().enumerate() {
            self.write_comma(idx == 0);
            self.write_escaped_str(key);
            self.write_colon();
            self.serialize_reserved(value, trailing_comma(idx, fields.len()));
        }
        self.end_container('}', fields.is_empty());
    }

    pub fn serialize(&mut self, value: &Content) {
        self.serialize_reserved(value, 0);
    }

    /// Serializes `value`, where `reserved` is the number of characters that
    /// will follow it on the same line (the trailing comma of an expanded
    /// container).  This only matters for [`Format::PrettyCompact`].
    fn serialize_reserved(&mut self, value: &Content, reserved: usize) {
        if self.format == Format::PrettyCompact && is_container(value) {
            if let Some(line) = self.try_single_line(value, reserved) {
                self.write_str(&line);
                return;
            }
        }
        match value {
            Content::Bool(true) => self.write_str("true"),
            Content::Bool(false) => self.write_str("false"),
            Content::U8(n) => write!(self.out, "{n}").unwrap(),
            Content::U16(n) => write!(self.out, "{n}").unwrap(),
            Content::U32(n) => write!(self.out, "{n}").unwrap(),
            Content::U64(n) => write!(self.out, "{n}").unwrap(),
            Content::U128(n) => write!(self.out, "{n}").unwrap(),
            Content::I8(n) => write!(self.out, "{n}").unwrap(),
            Content::I16(n) => write!(self.out, "{n}").unwrap(),
            Content::I32(n) => write!(self.out, "{n}").unwrap(),
            Content::I64(n) => write!(self.out, "{n}").unwrap(),
            Content::I128(n) => write!(self.out, "{n}").unwrap(),
            Content::F32(f) => self.write_float(f, f.is_finite()),
            Content::F64(f) => self.write_float(f, f.is_finite()),
            Content::Char(c) => self.write_escaped_str(&(*c).to_string()),
            Content::String(s) => self.write_escaped_str(s),
            Content::Bytes(bytes) => {
                self.start_container('[');
                for (idx, byte) in bytes.iter().enumerate() {
                    self.write_comma(idx == 0);
                    self.write_str(&byte.to_string());
                }
                self.end_container(']', bytes.is_empty());
            }
            Content::None | Content::Unit | Content::UnitStruct(_) => self.write_str("null"),
            Content::Some(content) => self.serialize_reserved(content, reserved),
            Content::UnitVariant(_, _, variant) => self.write_escaped_str(variant),
            Content::NewtypeStruct(_, content) => self.serialize_reserved(content, reserved),
            Content::NewtypeVariant(_, _, variant, content) => {
                self.start_container('{');
                self.write_comma(true);
                self.write_escaped_str(variant);
                self.write_colon();
                self.serialize(content);
                self.end_container('}', false);
            }
            Content::Seq(seq) | Content::Tuple(seq) | Content::TupleStruct(_, seq) => {
                self.serialize_array(seq);
            }
            Content::TupleVariant(_, _, variant, seq) => {
                self.start_container('{');
                self.write_comma(true);
                self.write_escaped_str(variant);
                self.write_colon();
                self.serialize_array(seq);
                self.end_container('}', false);
            }
            Content::Map(map) => {
                self.start_container('{');
                for (idx, (key, value)) in map.iter().enumerate() {
                    self.write_comma(idx == 0);
                    let real_key = key.resolve_inner();
                    if let Content::String(ref s) = real_key {
                        self.write_escaped_str(s);
                    } else if let Content::UnitVariant(_, _, variant) = real_key {
                        self.write_escaped_str(variant);
                    } else if let Some(num) = real_key.as_i64() {
                        self.write_escaped_str(&num.to_string());
                    } else if let Some(num) = real_key.as_i128() {
                        self.write_escaped_str(&num.to_string());
                    } else {
                        panic!("cannot serialize maps without string keys to JSON");
                    }
                    self.write_colon();
                    self.serialize_reserved(value, trailing_comma(idx, map.len()));
                }
                self.end_container('}', map.is_empty());
            }
            Content::Struct(_, fields) => {
                self.serialize_object(fields);
            }
            Content::StructVariant(_, _, variant, fields) => {
                self.start_container('{');
                self.write_comma(true);
                self.write_escaped_str(variant);
                self.write_colon();
                self.serialize_object(fields);
                self.end_container('}', false);
            }
        }
    }

    fn write_float(&mut self, n: impl Display, is_finite: bool) {
        if is_finite {
            let start = self.out.len();
            write!(self.out, "{n}").unwrap();
            // ensure the result has .0 for whole numbers to be round-trip safe
            if !self.out[start..].contains('.') {
                self.out.push_str(".0");
            }
        } else {
            self.write_str("null");
        }
    }

    /// Returns true if the output is laid out over multiple indented lines.
    fn is_pretty(&self) -> bool {
        matches!(self.format, Format::Pretty | Format::PrettyCompact)
    }

    /// The number of characters already written to the current line.
    fn current_column(&self) -> usize {
        let line_start = self.out.rfind('\n').map_or(0, |idx| idx + 1);
        self.out[line_start..].chars().count()
    }

    /// Renders `value` in [`Format::SingleLine`] style and returns it if that
    /// fits on the current line within [`COMPACT_MAX_CHARS`], counting the
    /// `reserved` characters that follow it.
    fn try_single_line(&self, value: &Content, reserved: usize) -> Option<String> {
        let mut ser = Serializer::new();
        ser.format = Format::SingleLine;
        ser.serialize(value);
        let line = ser.into_result();
        if self.current_column() + line.chars().count() + reserved <= COMPACT_MAX_CHARS {
            Some(line)
        } else {
            None
        }
    }

    fn write_str(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn write_char(&mut self, c: char) {
        self.out.push(c);
    }

    fn write_escaped_str(&mut self, value: &str) {
        self.write_char('"');

        let bytes = value.as_bytes();
        let mut start = 0;

        for (i, &byte) in bytes.iter().enumerate() {
            let escape = ESCAPE[byte as usize];
            if escape == 0 {
                continue;
            }

            if start < i {
                self.write_str(&value[start..i]);
            }

            match escape {
                self::BB => self.write_str("\\b"),
                self::TT => self.write_str("\\t"),
                self::NN => self.write_str("\\n"),
                self::FF => self.write_str("\\f"),
                self::RR => self.write_str("\\r"),
                self::QU => self.write_str("\\\""),
                self::BS => self.write_str("\\\\"),
                self::U => {
                    static HEX_DIGITS: [u8; 16] = *b"0123456789abcdef";
                    self.write_str("\\u00");
                    self.write_char(HEX_DIGITS[(byte >> 4) as usize] as char);
                    self.write_char(HEX_DIGITS[(byte & 0xF) as usize] as char);
                }
                _ => unreachable!(),
            }

            start = i + 1;
        }

        if start != bytes.len() {
            self.write_str(&value[start..]);
        }

        self.write_char('"');
    }
}

/// The number of characters a trailing comma takes up after the item at
/// `idx` in a container of `len` items.
fn trailing_comma(idx: usize, len: usize) -> usize {
    if idx + 1 < len {
        1
    } else {
        0
    }
}

/// Returns true if `value` serializes to a JSON array or object.  `Some` and
/// newtype structs are transparent and left to their inner value.
fn is_container(value: &Content) -> bool {
    matches!(
        value,
        Content::Bytes(_)
            | Content::Seq(_)
            | Content::Tuple(_)
            | Content::TupleStruct(_, _)
            | Content::TupleVariant(_, _, _, _)
            | Content::NewtypeVariant(_, _, _, _)
            | Content::Map(_)
            | Content::Struct(_, _)
            | Content::StructVariant(_, _, _, _)
    )
}

const BB: u8 = b'b'; // \x08
const TT: u8 = b't'; // \x09
const NN: u8 = b'n'; // \x0A
const FF: u8 = b'f'; // \x0C
const RR: u8 = b'r'; // \x0D
const QU: u8 = b'"'; // \x22
const BS: u8 = b'\\'; // \x5C
const U: u8 = b'u'; // \x00...\x1F except the ones above

// Lookup table of escape sequences. A value of b'x' at index i means that byte
// i is escaped as "\x" in JSON. A value of 0 means that byte i is not escaped.
#[rustfmt::skip]
static ESCAPE: [u8; 256] = [
    //  1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
    U,  U,  U,  U,  U,  U,  U,  U, BB, TT, NN,  U, FF, RR,  U,  U, // 0
    U,  U,  U,  U,  U,  U,  U,  U,  U,  U,  U,  U,  U,  U,  U,  U, // 1
    0,  0, QU,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // 2
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // 3
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // 4
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, BS,  0,  0,  0, // 5
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // 6
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // 7
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // 8
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // 9
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // A
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // B
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // C
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // D
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // E
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, // F
];

/// Serializes a value to JSON.
pub fn to_string(value: &Content) -> String {
    let mut ser = Serializer::new();
    ser.serialize(value);
    ser.into_result()
}

/// Serializes a value to JSON in a compact pretty format.
///
/// Every array or object that fits on the current line within
/// [`COMPACT_MAX_CHARS`] is written on a single line.  Anything larger is
/// expanded one element per line, as in [`to_string_pretty`], and the same
/// rule is then applied to each element.  As a result a value that fits on
/// one line is written exactly as before, and a value that does not is
/// written like [`to_string_pretty`] except that its small nested values
/// stay on one line.
#[allow(unused)]
pub fn to_string_compact(value: &Content) -> String {
    let mut ser = Serializer::new();
    ser.format = Format::PrettyCompact;
    ser.serialize(value);
    ser.into_result()
}

/// Serializes a value to JSON pretty
#[allow(unused)]
pub fn to_string_pretty(value: &Content) -> String {
    let mut ser = Serializer::new();
    ser.format = Format::Pretty;
    ser.serialize(value);
    ser.into_result()
}

#[test]
fn test_to_string() {
    let json = to_string(&Content::Map(vec![
        (
            Content::from("environments"),
            Content::Seq(vec![
                Content::from("development"),
                Content::from("production"),
            ]),
        ),
        (Content::from("cmdline"), Content::Seq(vec![])),
        (Content::from("extra"), Content::Map(vec![])),
    ]));
    crate::assert_snapshot!(&json, @r#"{"environments":["development","production"],"cmdline":[],"extra":{}}"#);
}

#[test]
fn test_to_string_pretty() {
    let json = to_string_pretty(&Content::Map(vec![
        (
            Content::from("environments"),
            Content::Seq(vec![
                Content::from("development"),
                Content::from("production"),
            ]),
        ),
        (Content::from("cmdline"), Content::Seq(vec![])),
        (Content::from("extra"), Content::Map(vec![])),
    ]));
    crate::assert_snapshot!(&json, @r#"
    {
      "environments": [
        "development",
        "production"
      ],
      "cmdline": [],
      "extra": {}
    }
    "#);
}

#[test]
fn test_to_string_compact() {
    let content = Content::Map(vec![
        (
            Content::from("pos"),
            Content::Seq(vec![0u32.into(), 14134u32.into()]),
        ),
        (Content::from("rule"), Content::from("file")),
    ]);
    crate::assert_snapshot!(to_string_compact(&content), @r#"{"pos": [0, 14134], "rule": "file"}"#);
}

#[test]
fn test_to_string_compact_nested() {
    let pair = |rule: &str| {
        Content::Struct(
            "Pair",
            vec![
                ("pos", Content::Seq(vec![0u32.into(), 14134u32.into()])),
                ("rule", Content::String(rule.to_string())),
            ],
        )
    };
    let content = Content::Struct(
        "File",
        vec![
            ("pos", Content::Seq(vec![0u32.into(), 14134u32.into()])),
            (
                "pairs",
                Content::Seq(vec![
                    pair("file"),
                    pair("version"),
                    pair("new_symbols"),
                    pair("bit_timing"),
                    pair("nodes"),
                    pair("value_tables"),
                ]),
            ),
            ("empty_array", Content::Seq(vec![])),
            ("empty_object", Content::Map(vec![])),
            ("bytes", Content::Bytes(b"hehe".to_vec())),
            (
                "some",
                Content::Some(Box::new(Content::Seq(vec![1u32.into(), 2u32.into()]))),
            ),
            (
                "newtype_variant",
                Content::NewtypeVariant(
                    "Enum",
                    0,
                    "variant_a",
                    Box::new(Content::Struct(
                        "Inner",
                        vec![("a", 1u32.into()), ("b", Content::None)],
                    )),
                ),
            ),
            (
                "tuple_variant",
                Content::TupleVariant(
                    "Enum",
                    1,
                    "variant_b",
                    vec![Content::from("a"), 1u32.into()],
                ),
            ),
            (
                "struct_variant",
                Content::StructVariant("Enum", 2, "variant_c", vec![("x", 1u32.into())]),
            ),
        ],
    );
    crate::assert_snapshot!(to_string_compact(&content), @r#"
    {
      "pos": [0, 14134],
      "pairs": [
        {"pos": [0, 14134], "rule": "file"},
        {"pos": [0, 14134], "rule": "version"},
        {"pos": [0, 14134], "rule": "new_symbols"},
        {"pos": [0, 14134], "rule": "bit_timing"},
        {"pos": [0, 14134], "rule": "nodes"},
        {"pos": [0, 14134], "rule": "value_tables"}
      ],
      "empty_array": [],
      "empty_object": {},
      "bytes": [104, 101, 104, 101],
      "some": [1, 2],
      "newtype_variant": {"variant_a": {"a": 1, "b": null}},
      "tuple_variant": {"variant_b": ["a", 1]},
      "struct_variant": {"variant_c": {"x": 1}}
    }
    "#);
}

#[test]
fn test_to_string_compact_line_width() {
    // 24 one-character strings render to exactly 120 characters, which fits
    // on one line.  One more and the array is expanded like the pretty format.
    let fits = Content::Seq((0..24).map(|_| Content::from("a")).collect());
    let line = to_string_compact(&fits);
    assert_eq!(line.chars().count(), COMPACT_MAX_CHARS);
    assert!(!line.contains('\n'));
    let too_long = Content::Seq((0..25).map(|_| Content::from("a")).collect());
    assert_eq!(to_string_compact(&too_long), to_string_pretty(&too_long));

    // Inside an expanded container the indentation and the trailing comma
    // count against the limit as well.  Both inner arrays render to 118
    // characters: the first one has to leave room for its comma, the last
    // one fits exactly.
    let inner = || Content::Seq(vec![Content::from("x".repeat(114))]);
    let content = Content::Seq(vec![inner(), inner()]);
    let rendered = to_string_compact(&content);
    for line in rendered.lines() {
        assert!(line.chars().count() <= COMPACT_MAX_CHARS);
    }
    crate::assert_snapshot!(rendered, @r#"
    [
      [
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
      ],
      ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
    ]
    "#);
}

#[test]
fn test_to_string_num_keys() {
    let content = Content::Map(vec![
        (Content::from(42u32), Content::from(true)),
        (Content::from(-23i32), Content::from(false)),
    ]);
    let json = to_string_pretty(&content);
    crate::assert_snapshot!(&json, @r#"
    {
      "42": true,
      "-23": false
    }
    "#);
}

#[test]
fn test_to_string_unit_variant_keys() {
    let content = Content::Map(vec![
        (Content::UnitVariant("MyEnum", 0, "A"), Content::from(true)),
        (Content::UnitVariant("MyEnum", 1, "B"), Content::from(false)),
    ]);
    let json = to_string_pretty(&content);
    crate::assert_snapshot!(&json, @r#"
    {
      "A": true,
      "B": false
    }
    "#);
}

#[test]
fn test_to_string_pretty_complex() {
    let content = Content::Map(vec![
        (
            Content::from("is_alive"),
            Content::NewtypeStruct("Some", Content::from(true).into()),
        ),
        (
            Content::from("newtype_variant"),
            Content::NewtypeVariant(
                "Foo",
                0,
                "variant_a",
                Box::new(Content::Struct(
                    "VariantA",
                    vec![
                        ("field_a", Content::String("value_a".into())),
                        ("field_b", 42u32.into()),
                    ],
                )),
            ),
        ),
        (
            Content::from("struct_variant"),
            Content::StructVariant(
                "Foo",
                0,
                "variant_b",
                vec![
                    ("field_a", Content::String("value_a".into())),
                    ("field_b", 42u32.into()),
                ],
            ),
        ),
        (
            Content::from("tuple_variant"),
            Content::TupleVariant(
                "Foo",
                0,
                "variant_c",
                vec![(Content::String("value_a".into())), (42u32.into())],
            ),
        ),
        (Content::from("empty_array"), Content::Seq(vec![])),
        (Content::from("empty_object"), Content::Map(vec![])),
        (Content::from("array"), Content::Seq(vec![true.into()])),
        (
            Content::from("object"),
            Content::Map(vec![("foo".into(), true.into())]),
        ),
        (
            Content::from("array_of_objects"),
            Content::Seq(vec![Content::Struct(
                "MyType",
                vec![
                    ("foo", Content::from("bar".to_string())),
                    ("bar", Content::from("xxx".to_string())),
                ],
            )]),
        ),
        (
            Content::from("unit_variant"),
            Content::UnitVariant("Stuff", 0, "value"),
        ),
        (Content::from("u8"), Content::U8(8)),
        (Content::from("u16"), Content::U16(16)),
        (Content::from("u32"), Content::U32(32)),
        (Content::from("u64"), Content::U64(64)),
        (Content::from("u128"), Content::U128(128)),
        (Content::from("i8"), Content::I8(8)),
        (Content::from("i16"), Content::I16(16)),
        (Content::from("i32"), Content::I32(32)),
        (Content::from("i64"), Content::I64(64)),
        (Content::from("i128"), Content::I128(128)),
        (Content::from("f32"), Content::F32(32.0)),
        (Content::from("f64"), Content::F64(64.0)),
        (Content::from("char"), Content::Char('A')),
        (Content::from("bytes"), Content::Bytes(b"hehe".to_vec())),
        (Content::from("null"), Content::None),
        (Content::from("unit"), Content::Unit),
        (
            Content::from("crazy_string"),
            Content::String((0u8..=126).map(|x| x as char).collect()),
        ),
    ]);
    let json = to_string_pretty(&content);

    crate::assert_snapshot!(&json, @r##"
    {
      "is_alive": true,
      "newtype_variant": {
        "variant_a": {
          "field_a": "value_a",
          "field_b": 42
        }
      },
      "struct_variant": {
        "variant_b": {
          "field_a": "value_a",
          "field_b": 42
        }
      },
      "tuple_variant": {
        "variant_c": [
          "value_a",
          42
        ]
      },
      "empty_array": [],
      "empty_object": {},
      "array": [
        true
      ],
      "object": {
        "foo": true
      },
      "array_of_objects": [
        {
          "foo": "bar",
          "bar": "xxx"
        }
      ],
      "unit_variant": "value",
      "u8": 8,
      "u16": 16,
      "u32": 32,
      "u64": 64,
      "u128": 128,
      "i8": 8,
      "i16": 16,
      "i32": 32,
      "i64": 64,
      "i128": 128,
      "f32": 32.0,
      "f64": 64.0,
      "char": "A",
      "bytes": [
        104,
        101,
        104,
        101
      ],
      "null": null,
      "unit": null,
      "crazy_string": "\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\b\t\n\u000b\f\r\u000e\u000f\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017\u0018\u0019\u001a\u001b\u001c\u001d\u001e\u001f !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~"
    }
    "##);
}
