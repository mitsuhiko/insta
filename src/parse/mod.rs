use std::convert::TryFrom;

use linked_hash_map::LinkedHashMap;

mod error;
pub use error::{Error, Result};

pub type Obj = LinkedHashMap<String, Value>;

#[derive(Clone, Debug)]
pub enum Number {
    Int(i64),
    Float(f64),
}

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Sequence(Vec<Value>),
    Obj(Obj),
}

pub fn pop_nullable_str(obj: &mut Obj, key: &str) -> Result<Option<String>> {
    match obj.remove(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        _ => Err(Error::UnexpectedDataType),
    }
}

pub fn pop_str(obj: &mut Obj, key: &str) -> Result<String> {
    match obj.remove(key) {
        None => Err(Error::MissingField),
        Some(Value::String(s)) => Ok(s),
        _ => Err(Error::UnexpectedDataType),
    }
}

pub fn pop_nullable_u32(obj: &mut Obj, key: &str) -> Result<Option<u32>> {
    match obj.remove(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(Number::Int(num))) => {
            let num = u32::try_from(num).map_err(|_| Error::NumberIsInvalidU32)?;
            Ok(Some(num))
        }
        Some(Value::Number(_)) => Err(Error::NumberIsInvalidU32),
        _ => Err(Error::UnexpectedDataType),
    }
}

pub fn pop_u32(obj: &mut Obj, key: &str) -> Result<u32> {
    match obj.remove(key) {
        None => Err(Error::MissingField),
        Some(Value::Number(Number::Int(num))) => {
            let num = u32::try_from(num).map_err(|_| Error::NumberIsInvalidU32)?;
            Ok(num)
        }
        Some(Value::Number(_)) => Err(Error::NumberIsInvalidU32),
        _ => Err(Error::UnexpectedDataType),
    }
}

impl Value {
    pub fn from_yaml(s: &str) -> Result<Self> {
        let mut raw_values =
            yaml_rust::YamlLoader::load_from_str(s).map_err(|_| Error::FailedParsingYaml)?;

        match (raw_values.pop(), raw_values.pop()) {
            (Some(raw_value), None) => Value::try_from(raw_value),
            _ => Err(Error::FailedParsingYaml),
        }
    }

    pub fn as_yaml(&self) -> String {
        let yaml_vals = yaml_rust::Yaml::from(self.to_owned());

        let mut buf = String::new();
        let mut emitter = yaml_rust::YamlEmitter::new(&mut buf);
        emitter.dump(&yaml_vals).unwrap();

        buf.push('\n');
        buf
    }

    pub fn from_json(s: &str) -> json::JsonResult<Self> {
        let raw_value = json::parse(s)?;
        Ok(Value::from(raw_value))
    }

    pub fn as_json(&self) -> String {
        let json_vals = json::JsonValue::from(self.to_owned());

        let mut buf = Vec::new();
        json_vals.write(&mut buf).unwrap();

        String::from_utf8(buf).expect("JSON is valid UTF-8")
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<u32> for Value {
    fn from(num: u32) -> Self {
        Self::from(i64::from(num))
    }
}

impl From<i64> for Value {
    fn from(num: i64) -> Self {
        Self::Number(Number::Int(num))
    }
}

impl From<f64> for Value {
    fn from(num: f64) -> Self {
        Self::Number(Number::Float(num))
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<Obj> for Value {
    fn from(obj: Obj) -> Self {
        Self::Obj(obj)
    }
}

impl TryFrom<yaml_rust::Yaml> for Value {
    type Error = Error;

    fn try_from(val: yaml_rust::Yaml) -> Result<Self> {
        match val {
            yaml_rust::Yaml::Null => Ok(Self::Null),
            yaml_rust::Yaml::Boolean(b) => Ok(Self::from(b)),
            yaml_rust::Yaml::Integer(num) => Ok(Self::from(num)),
            yaml_rust::Yaml::Real(real_str) => {
                let real: f64 = real_str.parse().unwrap();
                Ok(Self::from(real))
            }
            yaml_rust::Yaml::String(s) => Ok(Self::from(s)),
            yaml_rust::Yaml::Array(seq) => {
                let seq = seq.into_iter().map(Self::try_from).collect::<Result<_>>()?;
                Ok(Self::Sequence(seq))
            }
            yaml_rust::Yaml::Hash(map) => {
                let map = map
                    .into_iter()
                    .map(|(k, v)| {
                        let k = if let yaml_rust::Yaml::String(s) = k {
                            s
                        } else {
                            return Err(Error::YamlIsInvalidJson);
                        };
                        let v = Self::try_from(v)?;
                        Ok((k, v))
                    })
                    .collect::<Result<_>>()?;
                Ok(Self::Obj(map))
            }
            yaml_rust::Yaml::BadValue | yaml_rust::Yaml::Alias(_) => Err(Error::FailedParsingYaml),
        }
    }
}

impl From<Value> for yaml_rust::Yaml {
    fn from(val: Value) -> Self {
        match val {
            Value::Null => Self::Null,
            Value::Bool(b) => Self::Boolean(b),
            Value::Number(Number::Int(num)) => Self::Integer(num),
            Value::Number(Number::Float(num)) => Self::Real(num.to_string()),
            Value::String(s) => Self::String(s),
            Value::Sequence(seq) => Self::Array(seq.into_iter().map(Self::from).collect()),
            Value::Obj(obj) => {
                let obj = obj
                    .into_iter()
                    .map(|(k, v)| (Self::String(k), Self::from(v)))
                    .collect();
                Self::Hash(obj)
            }
        }
    }
}

impl From<json::JsonValue> for Value {
    fn from(val: json::JsonValue) -> Self {
        match val {
            json::JsonValue::Null => Self::Null,
            json::JsonValue::Boolean(b) => Self::Bool(b),
            json::JsonValue::Number(n) => match i64::try_from(n) {
                Ok(int) => Self::from(int),
                Err(_) => {
                    let f = f64::from(n);
                    Self::from(f)
                }
            },
            json::JsonValue::Short(s) => Self::String(s.to_string()),
            json::JsonValue::String(s) => Self::from(s),
            json::JsonValue::Array(arr) => {
                let arr = arr.into_iter().map(Self::from).collect();
                Self::Sequence(arr)
            }
            json::JsonValue::Object(map) => {
                let map = map
                    .iter()
                    .map(|(k, v)| (k.to_owned(), Self::from(v.to_owned())))
                    .collect();
                Self::Obj(map)
            }
        }
    }
}

impl From<Value> for json::JsonValue {
    fn from(val: Value) -> Self {
        match val {
            Value::Null => Self::Null,
            Value::Bool(b) => Self::from(b),
            Value::Number(Number::Int(num)) => Self::from(num),
            Value::Number(Number::Float(num)) => Self::from(num),
            Value::String(s) => Self::from(s),
            Value::Sequence(seq) => Self::Array(seq.into_iter().map(Self::from).collect()),
            Value::Obj(obj) => {
                let obj = obj.into_iter().map(|(k, v)| (k, Self::from(v))).collect();
                Self::Object(obj)
            }
        }
    }
}
