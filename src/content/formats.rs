use super::{Content, Error, Result};

use yaml_rust::{yaml::Hash as YamlObj, Yaml as YamlValue};

impl Content {
    pub(crate) fn as_json(&self) -> String {
        crate::content::json::to_string(self)
    }

    pub(crate) fn as_json_pretty(&self) -> String {
        crate::content::json::to_string_pretty(self)
    }

    // NOTE: Not implemented as `TryFrom<_>` because this is not generic and we want it to remain
    // private
    pub(crate) fn from_yaml(s: &str) -> Result<Self> {
        let mut blobs =
            yaml_rust::YamlLoader::load_from_str(s).map_err(|_| Error::FailedParsingYaml)?;

        match (blobs.pop(), blobs.pop()) {
            (Some(blob), None) => Self::from_yaml_blob(blob).map_err(Into::into),
            _ => Err(Error::FailedParsingYaml),
        }
    }

    fn from_yaml_blob(blob: YamlValue) -> Result<Self> {
        match blob {
            YamlValue::Null => Ok(Self::None),
            YamlValue::Boolean(b) => Ok(Self::from(b)),
            YamlValue::Integer(num) => Ok(Self::from(num)),
            YamlValue::Real(real_str) => {
                let real: f64 = real_str.parse().unwrap();
                Ok(Self::from(real))
            }
            YamlValue::String(s) => Ok(Self::from(s)),
            YamlValue::Array(seq) => {
                let seq = seq
                    .into_iter()
                    .map(Self::from_yaml_blob)
                    .collect::<Result<_>>()?;
                Ok(Self::Seq(seq))
            }
            YamlValue::Hash(obj) => {
                let obj = obj
                    .into_iter()
                    .map(|(k, v)| Ok((Self::from_yaml_blob(k)?, Self::from_yaml_blob(v)?)))
                    .collect::<Result<_>>()?;
                Ok(Self::Map(obj))
            }
            YamlValue::BadValue | YamlValue::Alias(_) => Err(Error::FailedParsingYaml),
        }
    }

    pub(crate) fn as_yaml(&self) -> String {
        let yaml_blob = self.as_yaml_blob();

        let mut buf = String::new();
        let mut emitter = yaml_rust::YamlEmitter::new(&mut buf);
        emitter.dump(&yaml_blob).unwrap();

        buf.push('\n');
        buf
    }

    fn as_yaml_blob(&self) -> YamlValue {
        fn translate_seq(seq: &[Content]) -> YamlValue {
            let seq = seq.iter().map(Content::as_yaml_blob).collect();
            YamlValue::Array(seq)
        }

        fn translate_fields(fields: &[(&str, Content)]) -> YamlValue {
            let fields = fields
                .iter()
                .map(|(k, v)| (YamlValue::String(k.to_string()), v.as_yaml_blob()))
                .collect();
            YamlValue::Hash(fields)
        }

        match self {
            Self::Bool(b) => YamlValue::Boolean(*b),
            Self::U8(n) => YamlValue::Integer(i64::from(*n)),
            Self::U16(n) => YamlValue::Integer(i64::from(*n)),
            Self::U32(n) => YamlValue::Integer(i64::from(*n)),
            Self::U64(n) => YamlValue::Real(n.to_string()),
            Self::U128(n) => YamlValue::Real(n.to_string()),
            Self::I8(n) => YamlValue::Integer(i64::from(*n)),
            Self::I16(n) => YamlValue::Integer(i64::from(*n)),
            Self::I32(n) => YamlValue::Integer(i64::from(*n)),
            Self::I64(n) => YamlValue::Integer(*n),
            Self::I128(n) => YamlValue::Real(n.to_string()),
            Self::F32(f) => YamlValue::Real(f.to_string()),
            Self::F64(f) => YamlValue::Real(f.to_string()),
            Self::Char(c) => YamlValue::String(c.to_string()),
            Self::String(s) => YamlValue::String(s.to_owned()),
            Self::Bytes(bytes) => {
                let bytes = bytes
                    .iter()
                    .map(|b| YamlValue::Integer(i64::from(*b)))
                    .collect();
                YamlValue::Array(bytes)
            }
            Self::None | Self::Unit | Self::UnitStruct(_) => YamlValue::Null,
            Self::Some(content) => content.as_yaml_blob(),
            Self::UnitVariant(_, _, variant) => YamlValue::String(variant.to_string()),
            Self::NewtypeStruct(_, content) => content.as_yaml_blob(),
            Self::NewtypeVariant(_, _, variant, content) => {
                let mut obj = YamlObj::new();
                obj.insert(
                    YamlValue::String(variant.to_string()),
                    content.as_yaml_blob(),
                );
                YamlValue::Hash(obj)
            }
            Self::Seq(seq) => translate_seq(seq),
            Self::Tuple(seq) => translate_seq(seq),
            Self::TupleStruct(_, seq) => translate_seq(seq),
            Self::TupleVariant(_, _, variant, seq) => {
                let mut obj = YamlObj::new();
                obj.insert(YamlValue::String(variant.to_string()), translate_seq(seq));
                YamlValue::Hash(obj)
            }
            Self::Map(map) => {
                let map = map
                    .iter()
                    .map(|(k, v)| (k.as_yaml_blob(), v.as_yaml_blob()))
                    .collect();

                YamlValue::Hash(map)
            }
            Self::Struct(_name, fields) => translate_fields(fields),
            Self::StructVariant(_, _, variant, fields) => {
                let mut obj = YamlObj::new();
                obj.insert(
                    YamlValue::String(variant.to_string()),
                    translate_fields(fields),
                );
                YamlValue::Hash(obj)
            }
        }
    }
}
