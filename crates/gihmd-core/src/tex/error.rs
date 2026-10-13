use std::path::PathBuf;

use serde::Serialize;

use crate::process::ProcessError;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TexError {
    #[error("No TeX distribution was found; set the TeX binary directory in settings.")]
    ToolchainNotFound { searched: Vec<PathBuf> },
    #[error("The configured TeX binary directory is missing required programs.")]
    ToolchainInvalid { dir: PathBuf, missing: Vec<String> },
    #[error("The project has no main file.")]
    MainFileMissing,
    #[error("The compile process could not be run.")]
    Process(#[from] ProcessError),
    #[error("The build output could not be read.")]
    Io {
        path: PathBuf,
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: std::io::Error,
    },
    #[error("SyncTeX produced output that could not be parsed.")]
    SynctexUnparsable { output: String },
    #[error("No PDF has been built yet.")]
    NoPdf,
}
