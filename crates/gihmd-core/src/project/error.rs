use std::path::PathBuf;

use serde::Serialize;

use crate::paths::PathError;
use crate::paths::RelPath;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProjectError {
    #[error("The project directory does not exist.")]
    RootMissing { root: PathBuf },
    #[error("The path is invalid.")]
    Path(#[from] PathError),
    #[error("The file does not exist.")]
    NotFound { path: RelPath },
    #[error("The path already exists.")]
    AlreadyExists { path: RelPath },
    #[error("The file is not valid UTF-8 text.")]
    NotUtf8 {
        path: RelPath,
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: std::string::FromUtf8Error,
    },
    #[error("The file changed on disk since it was read.")]
    StaleWrite { path: RelPath, mtime_ms: u64 },
    #[error("The file system operation failed.")]
    Io {
        path: RelPath,
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: std::io::Error,
    },
    #[error("The directory could not be listed.")]
    Listing {
        root: PathBuf,
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: std::io::Error,
    },
}
