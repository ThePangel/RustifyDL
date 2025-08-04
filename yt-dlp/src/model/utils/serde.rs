//! Serde utilities for serializing and deserializing data.

use serde::{Deserialize, Deserializer};

/// Fix issue with 'none' string in JSON.
pub fn json_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let string: Option<String> = Option::deserialize(deserializer)?;

    match string.as_deref() {
        Some("none") => Ok(None),
        _ => Ok(string),
    }
}
