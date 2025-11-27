use serde::{de::value::Error as ValueError, Serialize};
#[cfg(feature = "ron")]
use std::borrow::Cow;
#[cfg(feature = "toml")]
use {
    core::str::FromStr,
    toml_edit::{visit_mut::*, Value},
    toml_writer::ToTomlValue,
};

use crate::{
    content::{json, yaml, Content, ContentSerializer},
    settings::Settings,
};

pub enum SerializationFormat {
    #[cfg(feature = "csv")]
    Csv,
    #[cfg(feature = "ron")]
    Ron,
    #[cfg(feature = "toml")]
    Toml,
    Yaml,
    Json,
    JsonCompact,
}

#[derive(Debug)]
pub enum SnapshotLocation {
    Inline,
    File,
}

pub fn serialize_content(mut content: Content, format: SerializationFormat) -> String {
    content = Settings::with(|settings| {
        if settings.sort_maps() {
            content.sort_maps();
        }
        #[cfg(feature = "redactions")]
        {
            content = settings.apply_redactions(content);
        }
        content
    });

    match format {
        SerializationFormat::Yaml => yaml::to_string(&content)[4..].to_string(),
        SerializationFormat::Json => json::to_string_pretty(&content),
        SerializationFormat::JsonCompact => json::to_string_compact(&content),
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
            let mut buf = String::new();
            let mut config = ron::ser::PrettyConfig::new();
            config.new_line = Cow::Borrowed("\n");
            config.indentor = Cow::Borrowed("  ");
            config.struct_names = true;
            let mut serializer = ron::ser::Serializer::with_options(
                &mut buf,
                Some(config),
                &ron::options::Options::default(),
            )
            .unwrap();
            content.serialize(&mut serializer).unwrap();
            buf
        }
        #[cfg(feature = "toml")]
        SerializationFormat::Toml => {
            struct SingleQuoter;

            impl VisitMut for SingleQuoter {
                fn visit_value_mut(&mut self, node: &mut Value) {
                    if let Value::String(f) = node {
                        let builder = toml_writer::TomlStringBuilder::new(f.value().as_str());
                        let formatted = builder
                            .as_literal()
                            .unwrap_or(builder.as_default())
                            .to_toml_value();

                        if let Ok(value) = Value::from_str(&formatted) {
                            *node = value;
                        }
                    }

                    visit_value_mut(self, node);
                }
            }

            let mut dm = toml_edit::ser::to_document(&content).unwrap();
            let mut visitor = SingleQuoter;
            visitor.visit_document_mut(&mut dm);

            let mut rv = dm.to_string();
            if rv.ends_with('\n') {
                rv.truncate(rv.len() - 1);
            }
            rv
        }
    }
}

pub fn serialize_value<S: Serialize>(s: &S, format: SerializationFormat) -> String {
    let serializer = ContentSerializer::<ValueError>::new();
    let content = Serialize::serialize(s, serializer).unwrap();
    serialize_content(content, format)
}

#[cfg(feature = "redactions")]
pub fn serialize_value_redacted<S: Serialize>(
    s: &S,
    redactions: &[(crate::redaction::Selector, crate::redaction::Redaction)],
    format: SerializationFormat,
) -> String {
    let serializer = ContentSerializer::<ValueError>::new();
    let mut content = Serialize::serialize(s, serializer).unwrap();
    for (selector, redaction) in redactions {
        content = selector.redact(content, redaction);
    }
    serialize_content(content, format)
}

#[test]
fn test_yaml_serialization() {
    let yaml = serialize_content(
        Content::Map(vec![
            (
                Content::from("env"),
                Content::Seq(vec![
                    Content::from("ENVIRONMENT"),
                    Content::from("production"),
                ]),
            ),
            (
                Content::from("cmdline"),
                Content::Seq(vec![Content::from("my-tool"), Content::from("run")]),
            ),
        ]),
        SerializationFormat::Yaml,
    );
    crate::assert_snapshot!(&yaml, @r"
    env:
      - ENVIRONMENT
      - production
    cmdline:
      - my-tool
      - run
    ");

    let inline_yaml = serialize_content(
        Content::Map(vec![
            (
                Content::from("env"),
                Content::Seq(vec![
                    Content::from("ENVIRONMENT"),
                    Content::from("production"),
                ]),
            ),
            (
                Content::from("cmdline"),
                Content::Seq(vec![Content::from("my-tool"), Content::from("run")]),
            ),
        ]),
        SerializationFormat::Yaml,
    );
    crate::assert_snapshot!(&inline_yaml, @r"
    env:
      - ENVIRONMENT
      - production
    cmdline:
      - my-tool
      - run
    ");
}
