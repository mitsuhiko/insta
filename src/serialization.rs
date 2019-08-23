use ron;
use serde::Serialize;
use serde_json;
use serde_yaml;

pub enum SerializationFormat {
    Ron,
    Yaml,
    Json,
}

pub enum SnapshotLocation {
    Inline,
    File,
}

pub fn serialize_value<S: Serialize>(
    s: &S,
    format: SerializationFormat,
    location: SnapshotLocation,
) -> String {
    match format {
        SerializationFormat::Yaml => {
            let serialized = serde_yaml::to_string(s).unwrap();
            match location {
                SnapshotLocation::Inline => serialized.to_string(),
                SnapshotLocation::File => serialized[4..].to_string(),
            }
        }
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

#[cfg(feature = "redactions")]
pub fn serialize_value_redacted<S: Serialize>(
    s: &S,
    redactions: &[(crate::redaction::Selector, crate::content::Content)],
    format: SerializationFormat,
    location: SnapshotLocation,
) -> String {
    let serializer = crate::content::ContentSerializer::<serde::de::value::Error>::new();
    let mut value = Serialize::serialize(s, serializer).unwrap();
    for (selector, redaction) in redactions {
        value = selector.redact(value, &redaction);
    }
    serialize_value(&value, format, location)
}
