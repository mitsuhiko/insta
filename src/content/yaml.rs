use crate::content::{Content, Error};

use yaml_rust::{yaml::Hash as YamlObj, Yaml as YamlValue};

pub fn parse_str(s: &str) -> Result<Content, Error> {
    let mut blobs =
        yaml_rust::YamlLoader::load_from_str(s).map_err(|_| Error::FailedParsingYaml)?;

    match (blobs.pop(), blobs.pop()) {
        (Some(blob), None) => from_yaml_blob(blob).map_err(Into::into),
        _ => Err(Error::FailedParsingYaml),
    }
}

fn from_yaml_blob(blob: YamlValue) -> Result<Content, Error> {
    match blob {
        YamlValue::Null => Ok(Content::None),
        YamlValue::Boolean(b) => Ok(Content::from(b)),
        YamlValue::Integer(num) => Ok(Content::from(num)),
        YamlValue::Real(real_str) => {
            let real: f64 = real_str.parse().unwrap();
            Ok(Content::from(real))
        }
        YamlValue::String(s) => Ok(Content::from(s)),
        YamlValue::Array(seq) => {
            let seq = seq
                .into_iter()
                .map(from_yaml_blob)
                .collect::<Result<_, Error>>()?;
            Ok(Content::Seq(seq))
        }
        YamlValue::Hash(obj) => {
            let obj = obj
                .into_iter()
                .map(|(k, v)| Ok((from_yaml_blob(k)?, from_yaml_blob(v)?)))
                .collect::<Result<_, Error>>()?;
            Ok(Content::Map(obj))
        }
        YamlValue::BadValue | YamlValue::Alias(_) => Err(Error::FailedParsingYaml),
    }
}

pub fn to_string(content: &Content) -> String {
    let yaml_blob = to_yaml_value(content);

    let mut buf = String::new();
    let mut emitter = yaml_rust::YamlEmitter::new(&mut buf);
    emitter.dump(&yaml_blob).unwrap();

    if !buf.ends_with('\n') {
        buf.push('\n');
    }
    buf
}

fn to_yaml_value(content: &Content) -> YamlValue {
    fn translate_seq(seq: &[Content]) -> YamlValue {
        let seq = seq.iter().map(to_yaml_value).collect();
        YamlValue::Array(seq)
    }

    fn translate_fields(fields: &[(&str, Content)]) -> YamlValue {
        let fields = fields
            .iter()
            .map(|(k, v)| (YamlValue::String(k.to_string()), to_yaml_value(v)))
            .collect();
        YamlValue::Hash(fields)
    }

    match content {
        Content::Bool(b) => YamlValue::Boolean(*b),
        Content::U8(n) => YamlValue::Integer(i64::from(*n)),
        Content::U16(n) => YamlValue::Integer(i64::from(*n)),
        Content::U32(n) => YamlValue::Integer(i64::from(*n)),
        Content::U64(n) => YamlValue::Real(n.to_string()),
        Content::U128(n) => YamlValue::Real(n.to_string()),
        Content::I8(n) => YamlValue::Integer(i64::from(*n)),
        Content::I16(n) => YamlValue::Integer(i64::from(*n)),
        Content::I32(n) => YamlValue::Integer(i64::from(*n)),
        Content::I64(n) => YamlValue::Integer(*n),
        Content::I128(n) => YamlValue::Real(n.to_string()),
        Content::F32(f) => YamlValue::Real(f.to_string()),
        Content::F64(f) => YamlValue::Real(f.to_string()),
        Content::Char(c) => YamlValue::String(c.to_string()),
        Content::String(s) => YamlValue::String(s.to_owned()),
        Content::Bytes(bytes) => {
            let bytes = bytes
                .iter()
                .map(|b| YamlValue::Integer(i64::from(*b)))
                .collect();
            YamlValue::Array(bytes)
        }
        Content::None | Content::Unit | Content::UnitStruct(_) => YamlValue::Null,
        Content::Some(content) => to_yaml_value(content),
        Content::UnitVariant(_, _, variant) => YamlValue::String(variant.to_string()),
        Content::NewtypeStruct(_, content) => to_yaml_value(content),
        Content::NewtypeVariant(_, _, variant, content) => {
            let mut obj = YamlObj::new();
            obj.insert(
                YamlValue::String(variant.to_string()),
                to_yaml_value(content),
            );
            YamlValue::Hash(obj)
        }
        Content::Seq(seq) => translate_seq(seq),
        Content::Tuple(seq) => translate_seq(seq),
        Content::TupleStruct(_, seq) => translate_seq(seq),
        Content::TupleVariant(_, _, variant, seq) => {
            let mut obj = YamlObj::new();
            obj.insert(YamlValue::String(variant.to_string()), translate_seq(seq));
            YamlValue::Hash(obj)
        }
        Content::Map(map) => {
            let map = map
                .iter()
                .map(|(k, v)| (to_yaml_value(k), to_yaml_value(v)))
                .collect();

            YamlValue::Hash(map)
        }
        Content::Struct(_name, fields) => translate_fields(fields),
        Content::StructVariant(_, _, variant, fields) => {
            let mut obj = YamlObj::new();
            obj.insert(
                YamlValue::String(variant.to_string()),
                translate_fields(fields),
            );
            YamlValue::Hash(obj)
        }
    }
}
