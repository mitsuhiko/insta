use ron;
use serde::de::value::Error;
use serde::Serialize;
use serde_json;
use serde_yaml;

use crate::content::{Content, ContentSerializer};
use crate::redaction::Selector;

pub enum SerializationFormat {
    Ron,
    Yaml,
    Json,
}

pub fn serialize_value<S: Serialize>(s: &S, format: SerializationFormat) -> String {
    match format {
        SerializationFormat::Yaml => serde_yaml::to_string(s).unwrap()[4..].to_string(),
        SerializationFormat::Json => serde_json::to_string_pretty(s).unwrap(),
        SerializationFormat::Ron => {
            let mut serializer = ron::ser::Serializer::new(
                Some(ron::ser::PrettyConfig {
                    new_line: "\n".to_string(),
                    indentor: "  ".to_string(),
                    ..ron::ser::PrettyConfig::default()
                }),
                true,
            );
            s.serialize(&mut serializer).unwrap();
            serializer.into_output_string()
        }
    }
}

pub fn serialize_value_redacted<S: Serialize>(
    s: &S,
    redactions: &[(Selector, Content)],
    format: SerializationFormat,
) -> String {
    let serializer = ContentSerializer::<Error>::new();
    let mut value = Serialize::serialize(s, serializer).unwrap();
    for (selector, redaction) in redactions {
        value = selector.redact(value, &redaction);
    }
    serialize_value(&value, format)
}
