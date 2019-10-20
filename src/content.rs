// this module is based on the content module in serde::private::ser
use serde::ser::{self, Serialize, Serializer};
use std::marker::PhantomData;

/// Represents variable typed content.
///
/// This is used internally for the serialization system to
/// represent values before the actual snapshots are written.
#[derive(Debug, Clone)]
pub enum Content {
    Bool(bool),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),

    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),

    F32(f32),
    F64(f64),

    Char(char),
    String(String),
    Bytes(Vec<u8>),

    #[doc(hidden)]
    None,
    #[doc(hidden)]
    Some(Box<Content>),

    #[doc(hidden)]
    Unit,
    #[doc(hidden)]
    UnitStruct(&'static str),
    #[doc(hidden)]
    UnitVariant(&'static str, u32, &'static str),
    #[doc(hidden)]
    NewtypeStruct(&'static str, Box<Content>),
    #[doc(hidden)]
    NewtypeVariant(&'static str, u32, &'static str, Box<Content>),

    Seq(Vec<Content>),
    #[doc(hidden)]
    Tuple(Vec<Content>),
    #[doc(hidden)]
    TupleStruct(&'static str, Vec<Content>),
    #[doc(hidden)]
    TupleVariant(&'static str, u32, &'static str, Vec<Content>),
    Map(Vec<(Content, Content)>),
    #[doc(hidden)]
    Struct(&'static str, Vec<(&'static str, Content)>),
    #[doc(hidden)]
    StructVariant(
        &'static str,
        u32,
        &'static str,
        Vec<(&'static str, Content)>,
    ),
}

#[derive(PartialEq, PartialOrd, Debug)]
pub enum Key<'a> {
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(f64),
    Str(&'a str),
    Bytes(&'a [u8]),
    Other,
}

impl<'a> Eq for Key<'a> {}

impl<'a> Ord for Key<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Less)
    }
}

macro_rules! impl_from {
    ($ty:ty, $newty:ident) => {
        impl From<$ty> for Content {
            fn from(value: $ty) -> Content {
                Content::$newty(value)
            }
        }
    };
}

impl_from!(bool, Bool);
impl_from!(u8, U8);
impl_from!(u16, U16);
impl_from!(u32, U32);
impl_from!(u64, U64);
impl_from!(i8, I8);
impl_from!(i16, I16);
impl_from!(i32, I32);
impl_from!(i64, I64);
impl_from!(f32, F32);
impl_from!(f64, F64);
impl_from!(char, Char);
impl_from!(String, String);
impl_from!(Vec<u8>, Bytes);

impl From<()> for Content {
    fn from(_value: ()) -> Content {
        Content::Unit
    }
}

impl<'a> From<&'a str> for Content {
    fn from(value: &'a str) -> Content {
        Content::String(value.to_string())
    }
}

impl<'a> From<&'a [u8]> for Content {
    fn from(value: &'a [u8]) -> Content {
        Content::Bytes(value.to_vec())
    }
}

impl Content {
    /// Returns the value as string
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Content::String(ref s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub(crate) fn as_key(&self) -> Key<'_> {
        match *self {
            Content::Bool(val) => Key::Bool(val),
            Content::Char(val) => Key::U64(val as u64),
            Content::U16(val) => Key::U64(val.into()),
            Content::U32(val) => Key::U64(val.into()),
            Content::U64(val) => Key::U64(val),
            Content::I16(val) => Key::I64(val.into()),
            Content::I32(val) => Key::I64(val.into()),
            Content::I64(val) => Key::I64(val),
            Content::F32(val) => Key::F64(val.into()),
            Content::F64(val) => Key::F64(val),
            Content::String(ref val) => Key::Str(&val.as_str()),
            Content::Bytes(ref val) => Key::Bytes(&val[..]),
            Content::Some(ref val) => val.as_key(),
            _ => Key::Other,
        }
    }

    /// Returns the value as u64
    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            Content::U8(v) => Some(u64::from(v)),
            Content::U16(v) => Some(u64::from(v)),
            Content::U32(v) => Some(u64::from(v)),
            Content::U64(v) => Some(v),
            Content::I8(v) if v >= 0 => Some(v as u64),
            Content::I16(v) if v >= 0 => Some(v as u64),
            Content::I32(v) if v >= 0 => Some(v as u64),
            Content::I64(v) if v >= 0 => Some(v as u64),
            _ => None,
        }
    }

    pub(crate) fn sort_maps(&mut self) {
        self.walk(&mut |content| {
            if let Content::Map(ref mut items) = content {
                items.sort_by(|a, b| a.0.as_key().cmp(&b.0.as_key()));
            }
            true
        })
    }

    /// Recursively walks the content structure mutably.
    ///
    /// The callback is invoked for every content in the tree.
    pub fn walk<F: FnMut(&mut Content) -> bool>(&mut self, visit: &mut F) {
        if !visit(self) {
            return;
        }

        match *self {
            Content::Some(ref mut inner) => {
                Self::walk(&mut *inner, visit);
            }
            Content::NewtypeStruct(_, ref mut inner) => {
                Self::walk(&mut *inner, visit);
            }
            Content::NewtypeVariant(_, _, _, ref mut inner) => {
                Self::walk(&mut *inner, visit);
            }
            Content::Seq(ref mut vec) => {
                for inner in vec.iter_mut() {
                    Self::walk(inner, visit);
                }
            }
            Content::Map(ref mut vec) => {
                for inner in vec.iter_mut() {
                    Self::walk(&mut inner.0, visit);
                    Self::walk(&mut inner.1, visit);
                }
            }
            Content::Struct(_, ref mut vec) => {
                for inner in vec.iter_mut() {
                    Self::walk(&mut inner.1, visit);
                }
            }
            Content::StructVariant(_, _, _, ref mut vec) => {
                for inner in vec.iter_mut() {
                    Self::walk(&mut inner.1, visit);
                }
            }
            Content::Tuple(ref mut vec) => {
                for inner in vec.iter_mut() {
                    Self::walk(inner, visit);
                }
            }
            Content::TupleStruct(_, ref mut vec) => {
                for inner in vec.iter_mut() {
                    Self::walk(inner, visit);
                }
            }
            Content::TupleVariant(_, _, _, ref mut vec) => {
                for inner in vec.iter_mut() {
                    Self::walk(inner, visit);
                }
            }
            _ => {}
        }
    }
}

impl Serialize for Content {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Content::Bool(b) => serializer.serialize_bool(b),
            Content::U8(u) => serializer.serialize_u8(u),
            Content::U16(u) => serializer.serialize_u16(u),
            Content::U32(u) => serializer.serialize_u32(u),
            Content::U64(u) => serializer.serialize_u64(u),
            Content::I8(i) => serializer.serialize_i8(i),
            Content::I16(i) => serializer.serialize_i16(i),
            Content::I32(i) => serializer.serialize_i32(i),
            Content::I64(i) => serializer.serialize_i64(i),
            Content::F32(f) => serializer.serialize_f32(f),
            Content::F64(f) => serializer.serialize_f64(f),
            Content::Char(c) => serializer.serialize_char(c),
            Content::String(ref s) => serializer.serialize_str(s),
            Content::Bytes(ref b) => serializer.serialize_bytes(b),
            Content::None => serializer.serialize_none(),
            Content::Some(ref c) => serializer.serialize_some(&**c),
            Content::Unit => serializer.serialize_unit(),
            Content::UnitStruct(n) => serializer.serialize_unit_struct(n),
            Content::UnitVariant(n, i, v) => serializer.serialize_unit_variant(n, i, v),
            Content::NewtypeStruct(n, ref c) => serializer.serialize_newtype_struct(n, &**c),
            Content::NewtypeVariant(n, i, v, ref c) => {
                serializer.serialize_newtype_variant(n, i, v, &**c)
            }
            Content::Seq(ref elements) => elements.serialize(serializer),
            Content::Tuple(ref elements) => {
                use serde::ser::SerializeTuple;
                let mut tuple = serializer.serialize_tuple(elements.len())?;
                for e in elements {
                    tuple.serialize_element(e)?;
                }
                tuple.end()
            }
            Content::TupleStruct(n, ref fields) => {
                use serde::ser::SerializeTupleStruct;
                let mut ts = serializer.serialize_tuple_struct(n, fields.len())?;
                for f in fields {
                    ts.serialize_field(f)?;
                }
                ts.end()
            }
            Content::TupleVariant(n, i, v, ref fields) => {
                use serde::ser::SerializeTupleVariant;
                let mut tv = serializer.serialize_tuple_variant(n, i, v, fields.len())?;
                for f in fields {
                    tv.serialize_field(f)?;
                }
                tv.end()
            }
            Content::Map(ref entries) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for &(ref k, ref v) in entries {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            Content::Struct(n, ref fields) => {
                use serde::ser::SerializeStruct;
                let mut s = serializer.serialize_struct(n, fields.len())?;
                for &(k, ref v) in fields {
                    s.serialize_field(k, v)?;
                }
                s.end()
            }
            Content::StructVariant(n, i, v, ref fields) => {
                use serde::ser::SerializeStructVariant;
                let mut sv = serializer.serialize_struct_variant(n, i, v, fields.len())?;
                for &(k, ref v) in fields {
                    sv.serialize_field(k, v)?;
                }
                sv.end()
            }
        }
    }
}

pub struct ContentSerializer<E> {
    error: PhantomData<E>,
}

impl<E> ContentSerializer<E> {
    pub fn new() -> Self {
        ContentSerializer { error: PhantomData }
    }
}

impl<E> Serializer for ContentSerializer<E>
where
    E: ser::Error,
{
    type Ok = Content;
    type Error = E;

    type SerializeSeq = SerializeSeq<E>;
    type SerializeTuple = SerializeTuple<E>;
    type SerializeTupleStruct = SerializeTupleStruct<E>;
    type SerializeTupleVariant = SerializeTupleVariant<E>;
    type SerializeMap = SerializeMap<E>;
    type SerializeStruct = SerializeStruct<E>;
    type SerializeStructVariant = SerializeStructVariant<E>;

    fn serialize_bool(self, v: bool) -> Result<Content, E> {
        Ok(Content::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Content, E> {
        Ok(Content::I8(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Content, E> {
        Ok(Content::I16(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Content, E> {
        Ok(Content::I32(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Content, E> {
        Ok(Content::I64(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Content, E> {
        Ok(Content::U8(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Content, E> {
        Ok(Content::U16(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Content, E> {
        Ok(Content::U32(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Content, E> {
        Ok(Content::U64(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Content, E> {
        Ok(Content::F32(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Content, E> {
        Ok(Content::F64(v))
    }

    fn serialize_char(self, v: char) -> Result<Content, E> {
        Ok(Content::Char(v))
    }

    fn serialize_str(self, value: &str) -> Result<Content, E> {
        Ok(Content::String(value.to_owned()))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Content, E> {
        Ok(Content::Bytes(value.to_owned()))
    }

    fn serialize_none(self) -> Result<Content, E> {
        Ok(Content::None)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Content, E>
    where
        T: Serialize,
    {
        Ok(Content::Some(Box::new(value.serialize(self)?)))
    }

    fn serialize_unit(self) -> Result<Content, E> {
        Ok(Content::Unit)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Content, E> {
        Ok(Content::UnitStruct(name))
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Content, E> {
        Ok(Content::UnitVariant(name, variant_index, variant))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Content, E>
    where
        T: Serialize,
    {
        Ok(Content::NewtypeStruct(
            name,
            Box::new(value.serialize(self)?),
        ))
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Content, E>
    where
        T: Serialize,
    {
        Ok(Content::NewtypeVariant(
            name,
            variant_index,
            variant,
            Box::new(value.serialize(self)?),
        ))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, E> {
        Ok(SerializeSeq {
            elements: Vec::with_capacity(len.unwrap_or(0)),
            error: PhantomData,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, E> {
        Ok(SerializeTuple {
            elements: Vec::with_capacity(len),
            error: PhantomData,
        })
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, E> {
        Ok(SerializeTupleStruct {
            name,
            fields: Vec::with_capacity(len),
            error: PhantomData,
        })
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, E> {
        Ok(SerializeTupleVariant {
            name,
            variant_index,
            variant,
            fields: Vec::with_capacity(len),
            error: PhantomData,
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, E> {
        Ok(SerializeMap {
            entries: Vec::with_capacity(len.unwrap_or(0)),
            key: None,
            error: PhantomData,
        })
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, E> {
        Ok(SerializeStruct {
            name,
            fields: Vec::with_capacity(len),
            error: PhantomData,
        })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, E> {
        Ok(SerializeStructVariant {
            name,
            variant_index,
            variant,
            fields: Vec::with_capacity(len),
            error: PhantomData,
        })
    }
}

pub struct SerializeSeq<E> {
    elements: Vec<Content>,
    error: PhantomData<E>,
}

impl<E> ser::SerializeSeq for SerializeSeq<E>
where
    E: ser::Error,
{
    type Ok = Content;
    type Error = E;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), E>
    where
        T: Serialize,
    {
        let value = value.serialize(ContentSerializer::<E>::new())?;
        self.elements.push(value);
        Ok(())
    }

    fn end(self) -> Result<Content, E> {
        Ok(Content::Seq(self.elements))
    }
}

pub struct SerializeTuple<E> {
    elements: Vec<Content>,
    error: PhantomData<E>,
}

impl<E> ser::SerializeTuple for SerializeTuple<E>
where
    E: ser::Error,
{
    type Ok = Content;
    type Error = E;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), E>
    where
        T: Serialize,
    {
        let value = value.serialize(ContentSerializer::<E>::new())?;
        self.elements.push(value);
        Ok(())
    }

    fn end(self) -> Result<Content, E> {
        Ok(Content::Tuple(self.elements))
    }
}

pub struct SerializeTupleStruct<E> {
    name: &'static str,
    fields: Vec<Content>,
    error: PhantomData<E>,
}

impl<E> ser::SerializeTupleStruct for SerializeTupleStruct<E>
where
    E: ser::Error,
{
    type Ok = Content;
    type Error = E;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), E>
    where
        T: Serialize,
    {
        let value = value.serialize(ContentSerializer::<E>::new())?;
        self.fields.push(value);
        Ok(())
    }

    fn end(self) -> Result<Content, E> {
        Ok(Content::TupleStruct(self.name, self.fields))
    }
}

pub struct SerializeTupleVariant<E> {
    name: &'static str,
    variant_index: u32,
    variant: &'static str,
    fields: Vec<Content>,
    error: PhantomData<E>,
}

impl<E> ser::SerializeTupleVariant for SerializeTupleVariant<E>
where
    E: ser::Error,
{
    type Ok = Content;
    type Error = E;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), E>
    where
        T: Serialize,
    {
        let value = value.serialize(ContentSerializer::<E>::new())?;
        self.fields.push(value);
        Ok(())
    }

    fn end(self) -> Result<Content, E> {
        Ok(Content::TupleVariant(
            self.name,
            self.variant_index,
            self.variant,
            self.fields,
        ))
    }
}

pub struct SerializeMap<E> {
    entries: Vec<(Content, Content)>,
    key: Option<Content>,
    error: PhantomData<E>,
}

impl<E> ser::SerializeMap for SerializeMap<E>
where
    E: ser::Error,
{
    type Ok = Content;
    type Error = E;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), E>
    where
        T: Serialize,
    {
        let key = key.serialize(ContentSerializer::<E>::new())?;
        self.key = Some(key);
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), E>
    where
        T: Serialize,
    {
        let key = self
            .key
            .take()
            .expect("serialize_value called before serialize_key");
        let value = value.serialize(ContentSerializer::<E>::new())?;
        self.entries.push((key, value));
        Ok(())
    }

    fn end(self) -> Result<Content, E> {
        Ok(Content::Map(self.entries))
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(&mut self, key: &K, value: &V) -> Result<(), E>
    where
        K: Serialize,
        V: Serialize,
    {
        let key = key.serialize(ContentSerializer::<E>::new())?;
        let value = value.serialize(ContentSerializer::<E>::new())?;
        self.entries.push((key, value));
        Ok(())
    }
}

pub struct SerializeStruct<E> {
    name: &'static str,
    fields: Vec<(&'static str, Content)>,
    error: PhantomData<E>,
}

impl<E> ser::SerializeStruct for SerializeStruct<E>
where
    E: ser::Error,
{
    type Ok = Content;
    type Error = E;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), E>
    where
        T: Serialize,
    {
        let value = value.serialize(ContentSerializer::<E>::new())?;
        self.fields.push((key, value));
        Ok(())
    }

    fn end(self) -> Result<Content, E> {
        Ok(Content::Struct(self.name, self.fields))
    }
}

pub struct SerializeStructVariant<E> {
    name: &'static str,
    variant_index: u32,
    variant: &'static str,
    fields: Vec<(&'static str, Content)>,
    error: PhantomData<E>,
}

impl<E> ser::SerializeStructVariant for SerializeStructVariant<E>
where
    E: ser::Error,
{
    type Ok = Content;
    type Error = E;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), E>
    where
        T: Serialize,
    {
        let value = value.serialize(ContentSerializer::<E>::new())?;
        self.fields.push((key, value));
        Ok(())
    }

    fn end(self) -> Result<Content, E> {
        Ok(Content::StructVariant(
            self.name,
            self.variant_index,
            self.variant,
            self.fields,
        ))
    }
}
