use std::{collections::BTreeMap, convert::TryFrom};

use super::{Content, Error, Result};

pub fn into_unordered_struct_fields(
    map: Vec<(Content, Content)>,
) -> Result<BTreeMap<String, Content>> {
    map.into_iter()
        .map(|(k, v)| {
            if let Content::String(s) = k {
                Ok((s, v))
            } else {
                Err(Error::InvalidStructField(k))
            }
        })
        .collect()
}

pub fn pop_str(map: &mut BTreeMap<String, Content>, key: &str) -> Result<String> {
    match pop_nullable_str(map, key) {
        Err(e) => Err(e),
        Ok(None) => Err(Error::UnexpectedDataType),
        Ok(Some(s)) => Ok(s),
    }
}

pub fn pop_nullable_str(map: &mut BTreeMap<String, Content>, key: &str) -> Result<Option<String>> {
    match map.remove(key) {
        None => Ok(None),
        Some(content) => {
            if content.is_nil() {
                Ok(None)
            } else {
                match content.as_str() {
                    Some(s) => Ok(Some(s.to_owned())),
                    None => Err(Error::UnexpectedDataType),
                }
            }
        }
    }
}

pub fn pop_u32(map: &mut BTreeMap<String, Content>, key: &str) -> Result<u32> {
    match pop_nullable_u32(map, key) {
        Err(e) => Err(e),
        Ok(None) => Err(Error::UnexpectedDataType),
        Ok(Some(num)) => Ok(num),
    }
}

pub fn pop_nullable_u32(map: &mut BTreeMap<String, Content>, key: &str) -> Result<Option<u32>> {
    match map.remove(key) {
        None => Ok(None),
        Some(content) => {
            if content.is_nil() {
                Ok(None)
            } else {
                match content.as_i64().and_then(|n| u32::try_from(n).ok()) {
                    Some(n) => Ok(Some(n)),
                    None => Err(Error::UnexpectedDataType),
                }
            }
        }
    }
}
