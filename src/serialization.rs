use serde::Serialize;
use serde_yaml;

use crate::redaction::Selector;
use serde_yaml::Value;

pub fn serialize_value<S: Serialize>(s: &S) -> String {
    serde_yaml::to_string(s).unwrap()
}

pub fn serialize_value_redacted<S: Serialize>(s: &S, redactions: &[(Selector, Value)]) -> String {
    let mut value = serde_yaml::to_value(s).unwrap();
    for (selector, redaction) in redactions {
        value = selector.redact(value, &redaction);
    }
    serde_yaml::to_string(&value).unwrap()[4..].to_string()
}
