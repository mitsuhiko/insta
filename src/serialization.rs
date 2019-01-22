use serde::Serialize;
use serde::de::value::Error;
use serde_yaml;

use crate::content::{Content, ContentSerializer};
use crate::redaction::Selector;

pub fn serialize_value<S: Serialize>(s: &S) -> String {
    serde_yaml::to_string(s).unwrap()
}

pub fn serialize_value_redacted<S: Serialize>(s: &S, redactions: &[(Selector, Content)]) -> String {
    let serializer = ContentSerializer::<Error>::new();
    let mut value = Serialize::serialize(s, serializer).unwrap();
    for (selector, redaction) in redactions {
        value = selector.redact(value, &redaction);
    }
    serde_yaml::to_string(&value).unwrap()[4..].to_string()
}
