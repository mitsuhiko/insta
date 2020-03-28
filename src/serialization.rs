use serde::de::value::Error as ValueError;
use serde::Serialize;

use crate::content::{Content, ContentSerializer};
use crate::settings::Settings;

pub enum SerializationFormat {
    #[cfg(feature = "ron")]
    Ron,
    Yaml,
    Json,
}

pub enum SnapshotLocation {
    Inline,
    File,
}

pub fn serialize_content(
    mut content: Content,
    format: SerializationFormat,
    location: SnapshotLocation,
) -> String {
    content = Settings::with(|settings| {
        if settings.sort_maps() {
            content.sort_maps();
        }
        #[cfg(feature = "redactions")]
        {
            for (selector, redaction) in settings.iter_redactions() {
                content = selector.redact(content, redaction);
            }
        }
        content
    });

    match format {
        SerializationFormat::Yaml => {
            let serialized = serde_yaml::to_string(&content).unwrap();
            match location {
                SnapshotLocation::Inline => serialized,
                SnapshotLocation::File => serialized[4..].to_string(),
            }
        }
        SerializationFormat::Json => serde_json::to_string_pretty(&content).unwrap(),
        #[cfg(feature = "ron")]
        SerializationFormat::Ron => {
            let mut serializer = ron::ser::Serializer::new(
                Some(ron::ser::PrettyConfig {
                    new_line: "\n".to_string(),
                    indentor: "  ".to_string(),
                    ..ron::ser::PrettyConfig::default()
                }),
                true,
            );
            content.serialize(&mut serializer).unwrap();
            serializer.into_output_string()
        }
    }
}

pub fn serialize_value<S: Serialize>(
    s: &S,
    format: SerializationFormat,
    location: SnapshotLocation,
) -> String {
    let serializer = ContentSerializer::<ValueError>::new();
    let content = Serialize::serialize(s, serializer).unwrap();
    serialize_content(content, format, location)
}

#[cfg(feature = "redactions")]
pub fn serialize_value_redacted<S: Serialize>(
    s: &S,
    redactions: &[(crate::redaction::Selector, crate::redaction::Redaction)],
    format: SerializationFormat,
    location: SnapshotLocation,
) -> String {
    let serializer = ContentSerializer::<ValueError>::new();
    let mut content = Serialize::serialize(s, serializer).unwrap();
    for (selector, redaction) in redactions {
        content = selector.redact(content, &redaction);
    }
    serialize_content(content, format, location)
}
