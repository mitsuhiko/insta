use serde::de::value::Error as ValueError;
use serde::Serialize;

use crate::content::{Content, ContentSerializer};
use crate::settings::Settings;

pub enum SerializationFormat {
    #[cfg(feature = "csv")]
    Csv,
    #[cfg(feature = "ron")]
    Ron,
    #[cfg(feature = "toml")]
    Toml,
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
        #[cfg(feature = "csv")]
        SerializationFormat::Csv => {
            let mut buf = Vec::with_capacity(128);
            {
                let mut writer = csv::Writer::from_writer(&mut buf);
                // if the top-level content we're serializing is a vector we
                // want to serialize it multiple times once for each item.
                if let Some(content_slice) = content.as_slice() {
                    for content in content_slice {
                        writer.serialize(content).unwrap();
                    }
                } else {
                    writer.serialize(&content).unwrap();
                }
                writer.flush().unwrap();
            }
            if buf.ends_with(b"\n") {
                buf.truncate(buf.len() - 1);
            }
            String::from_utf8(buf).unwrap()
        }
        #[cfg(feature = "ron")]
        SerializationFormat::Ron => {
            let mut buf = Vec::new();
            let mut config = ron::ser::PrettyConfig::new();
            config.new_line = "\n".to_string();
            config.indentor = "  ".to_string();
            let mut serializer = ron::ser::Serializer::new(&mut buf, Some(config), true).unwrap();
            content.serialize(&mut serializer).unwrap();
            String::from_utf8(buf).unwrap()
        }
        #[cfg(feature = "toml")]
        SerializationFormat::Toml => {
            let mut rv = toml::to_string_pretty(&content).unwrap();
            if rv.ends_with('\n') {
                rv.truncate(rv.len() - 1);
            }
            rv
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
