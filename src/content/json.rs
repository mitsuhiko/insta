use std::fmt::{Display, Write};

use crate::content::Content;

pub fn format_float<T: Display>(value: T) -> String {
    let mut rv = format!("{}", value);
    if !rv.contains('.') {
        rv.push_str(".0");
    }
    rv
}

/// Serializes a serializable to JSON.
pub struct Serializer {
    out: String,
    pretty: bool,
    indentation: usize,
}

impl Serializer {
    /// Creates a new serializer that writes into the given writer.
    pub fn new() -> Serializer {
        Serializer {
            out: String::new(),
            pretty: false,
            indentation: 0,
        }
    }

    pub fn into_result(self) -> String {
        self.out
    }

    fn write_indentation(&mut self) {
        if self.pretty {
            write!(self.out, "{: ^1$}", "", self.indentation * 2).unwrap();
        }
    }

    fn start_container(&mut self, c: char) {
        self.write_char(c);
        self.indentation += 1;
    }

    fn end_container(&mut self, c: char, empty: bool) {
        self.indentation -= 1;
        if self.pretty && !empty {
            self.write_char('\n');
            self.write_indentation();
        }
        self.write_char(c);
    }

    fn write_comma(&mut self, first: bool) {
        if self.pretty {
            if first {
                self.write_char('\n');
            } else {
                self.write_str(",\n");
            }
            self.write_indentation();
        } else if !first {
            self.write_char(',');
        }
    }

    fn write_colon(&mut self) {
        if self.pretty {
            self.write_str(": ");
        } else {
            self.write_char(':');
        }
    }

    fn serialize_array(&mut self, items: &[Content]) {
        self.start_container('[');
        for (idx, item) in items.iter().enumerate() {
            self.write_comma(idx == 0);
            self.serialize(item);
        }
        self.end_container(']', items.is_empty());
    }

    fn serialize_object(&mut self, fields: &[(&str, Content)]) {
        self.start_container('{');
        for (idx, (key, value)) in fields.iter().enumerate() {
            self.write_comma(idx == 0);
            self.write_escaped_str(key);
            self.write_colon();
            self.serialize(value);
        }
        self.end_container('}', fields.is_empty());
    }

    pub fn serialize(&mut self, value: &Content) {
        match value {
            Content::Bool(true) => self.write_str("true"),
            Content::Bool(false) => self.write_str("false"),
            Content::U8(n) => self.write_str(&n.to_string()),
            Content::U16(n) => self.write_str(&n.to_string()),
            Content::U32(n) => self.write_str(&n.to_string()),
            Content::U64(n) => self.write_str(&n.to_string()),
            Content::U128(n) => self.write_str(&n.to_string()),
            Content::I8(n) => self.write_str(&n.to_string()),
            Content::I16(n) => self.write_str(&n.to_string()),
            Content::I32(n) => self.write_str(&n.to_string()),
            Content::I64(n) => self.write_str(&n.to_string()),
            Content::I128(n) => self.write_str(&n.to_string()),
            Content::F32(f) => {
                if f.is_finite() {
                    self.write_str(&format_float(f));
                } else {
                    self.write_str("null")
                }
            }
            Content::F64(f) => {
                if f.is_finite() {
                    self.write_str(&format_float(f));
                } else {
                    self.write_str("null")
                }
            }
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
            Content::Some(content) => self.serialize(content),
            Content::UnitVariant(_, _, variant) => self.write_escaped_str(variant),
            Content::NewtypeStruct(_, content) => self.serialize(content),
            Content::NewtypeVariant(_, _, variant, content) => {
                self.start_container('{');
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
                self.write_escaped_str(variant);
                self.write_colon();
                self.serialize_array(seq);
                self.end_container('}', false);
            }
            Content::Map(map) => {
                self.start_container('{');
                for (idx, (key, value)) in map.iter().enumerate() {
                    self.write_comma(idx == 0);
                    if let Content::String(ref s) = key {
                        self.write_escaped_str(s);
                    } else {
                        panic!("cannot serialize maps without string keys to JSON");
                    }
                    self.write_colon();
                    self.serialize(value);
                }
                self.end_container('}', map.is_empty());
            }
            Content::Struct(_, fields) => {
                self.serialize_object(fields);
            }
            Content::StructVariant(_, _, variant, fields) => {
                self.start_container('{');
                self.write_escaped_str(variant);
                self.write_colon();
                self.serialize_object(fields);
                self.end_container('}', false);
            }
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

/// Serializes a value to JSON pretty
pub fn to_string_pretty(value: &Content) -> String {
    let mut ser = Serializer::new();
    ser.pretty = true;
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
    crate::assert_snapshot!(&json, @r###"{"environments":["development","production"],"cmdline":[],"extra":{}}"###);
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
    crate::assert_snapshot!(&json, @r###"
    {
      "environments": [
        "development",
        "production"
      ],
      "cmdline": [],
      "extra": {}
    }
    "###);
}

#[test]
fn test_to_string_pretty_complex() {
    let content = Content::Map(vec![
        (
            Content::from("is_alive"),
            Content::NewtypeStruct("Some", Content::from(true).into()),
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

    crate::assert_snapshot!(&json, @r###"
    {
      "is_alive": true,
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
    "###);
}
