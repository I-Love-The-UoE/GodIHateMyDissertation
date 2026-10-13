use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConfigError {
    #[error("The configuration file could not be read or written.")]
    Io {
        path: PathBuf,
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: std::io::Error,
    },
    #[error("The configuration file is not valid JSON.")]
    Malformed {
        path: PathBuf,
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: serde_json::Error,
    },
    #[error("The setting has an invalid value.")]
    Invalid { setting: String, detail: String },
}
