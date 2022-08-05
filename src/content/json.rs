use crate::content::Content;

/// Serializes a serializable to JSON.
pub struct Serializer {
    out: String,
}

impl Serializer {
    /// Creates a new serializer that writes into the given writer.
    pub fn new() -> Serializer {
        Serializer { out: String::new() }
    }

    pub fn into_result(self) -> String {
        self.out
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
                    self.write_str(f.to_string().as_str())
                } else {
                    self.write_str("null")
                }
            }
            Content::F64(f) => {
                if f.is_finite() {
                    self.write_str(f.to_string().as_str())
                } else {
                    self.write_str("null")
                }
            }
            Content::Char(c) => self.write_escaped_str(&(*c as u32).to_string()),
            Content::String(s) => self.write_escaped_str(&s),
            Content::Bytes(bytes) => {
                self.write_char('[');
                for (idx, byte) in bytes.iter().enumerate() {
                    if idx > 0 {
                        self.write_char(',');
                    }
                    self.write_str(&byte.to_string());
                }
            }
            Content::None | Content::Unit | Content::UnitStruct(_) => self.write_str("null"),
            Content::Some(content) => self.serialize(content),
            Content::UnitVariant(_, _, variant) => self.write_escaped_str(variant),
            Content::NewtypeStruct(_, content) => self.serialize(content),
            Content::NewtypeVariant(_, _, variant, content) => {
                self.write_char('{');
                self.write_escaped_str(variant);
                self.write_char(':');
                self.serialize(content);
                self.write_char('}');
            }
            Content::Seq(seq) | Content::Tuple(seq) | Content::TupleStruct(_, seq) => {
                self.write_char('[');
                for (idx, item) in seq.iter().enumerate() {
                    if idx > 0 {
                        self.write_char(',');
                    }
                    self.serialize(item);
                }
                self.write_char(']');
            }
            Content::TupleVariant(_, _, variant, seq) => {
                self.write_char('{');
                self.write_escaped_str(variant);
                self.write_char(':');
                self.write_char('[');
                for (idx, item) in seq.iter().enumerate() {
                    if idx > 0 {
                        self.write_char(',');
                    }
                    self.serialize(item);
                }
                self.write_char(']');
                self.write_char('}');
            }
            Content::Map(map) => {
                self.write_char('{');
                for (idx, (key, value)) in map.iter().enumerate() {
                    if idx > 0 {
                        self.write_char(',');
                    }
                    if let Content::String(ref s) = key {
                        self.write_escaped_str(s);
                    } else {
                        panic!("cannot serialize maps without string keys to JSON");
                    }
                    self.write_char(':');
                    self.serialize(value);
                }
                self.write_char('}');
            }
            Content::Struct(_, fields) => {
                self.write_char('{');
                for (idx, (key, value)) in fields.iter().enumerate() {
                    if idx > 0 {
                        self.write_char(',');
                    }
                    self.write_escaped_str(key);
                    self.write_char(':');
                    self.serialize(value);
                }
                self.write_char('}');
            }
            Content::StructVariant(_, _, variant, fields) => {
                self.write_char('{');
                self.write_escaped_str(variant);
                self.write_char(':');
                for (idx, (key, value)) in fields.iter().enumerate() {
                    if idx > 0 {
                        self.write_char(',');
                    }
                    self.write_escaped_str(key);
                    self.write_char(':');
                    self.serialize(value);
                }
                self.write_char('}');
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
