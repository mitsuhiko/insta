use std::error::Error;
use std::ffi::OsStr;
use std::io::{self, BufRead};

use crate::{MetaData, Snapshot};

pub(crate) trait SnapfileFormatter {
    fn serialize(snapshot: &Snapshot, f: &mut dyn io::Write) -> Result<(), Box<dyn Error>>;
    fn deserialize(f: &mut dyn BufRead, fname: &OsStr) -> Result<Snapshot, Box<dyn Error>>;
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DefaultSnapfileFormatter;

impl SnapfileFormatter for DefaultSnapfileFormatter {
    fn serialize(snapshot: &Snapshot, mut f: &mut dyn io::Write) -> Result<(), Box<dyn Error>> {
        serde_yaml::to_writer(&mut f, &snapshot.metadata)?;
        f.write_all(b"\n---\n")?;
        f.write_all(snapshot.contents_str().as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
    }

    fn deserialize(mut f: &mut dyn BufRead, fname: &OsStr) -> Result<Snapshot, Box<dyn Error>> {
        let mut buf = String::new();

        f.read_line(&mut buf)?;

        // yaml format
        let metadata = if buf.trim_end() == "---" {
            loop {
                let read = f.read_line(&mut buf)?;
                if read == 0 {
                    break;
                }
                if buf[buf.len() - read..].trim_end() == "---" {
                    buf.truncate(buf.len() - read);
                    break;
                }
            }
            serde_yaml::from_str(&buf)?
        // legacy format
        } else {
            let mut rv = MetaData::default();
            loop {
                buf.clear();
                let read = f.read_line(&mut buf)?;
                if read == 0 || buf.trim_end().is_empty() {
                    buf.truncate(buf.len() - read);
                    break;
                }
                let mut iter = buf.splitn(2, ':');
                if let Some(key) = iter.next() {
                    if let Some(value) = iter.next() {
                        let value = value.trim();
                        match key.to_lowercase().as_str() {
                            "expression" => rv.expression = Some(value.to_string()),
                            "source" => rv.source = Some(value.into()),
                            _ => {}
                        }
                    }
                }
            }
            rv
        };

        buf.clear();
        for (idx, line) in (&mut f).lines().enumerate() {
            let line = line?;
            if idx > 0 {
                buf.push('\n');
            }
            buf.push_str(&line);
        }

        let module_name = fname
            .to_str()
            .unwrap_or("")
            .split("__")
            .next()
            .unwrap_or("<unknown>")
            .to_string();

        let snapshot_name = fname
            .to_str()
            .unwrap_or("")
            .split('.')
            .next()
            .unwrap_or("")
            .splitn(2, "__")
            .nth(1)
            .map(|x| x.to_string());

        Ok(Snapshot::from_components(
            module_name,
            snapshot_name,
            metadata,
            buf.into(),
        ))
    }
}
