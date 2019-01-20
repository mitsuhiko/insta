use serde::Serialize;
use serde_yaml;

pub fn serialize_value<S: Serialize>(s: &S) -> String {
    serde_yaml::to_string(s).unwrap()
}