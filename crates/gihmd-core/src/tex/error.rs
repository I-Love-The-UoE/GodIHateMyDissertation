use std::path::PathBuf;

use serde::Serialize;

use crate::download::DownloadError;
use crate::process::ProcessError;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TexError {
    #[error("The managed TeX Live is not installed yet.")]
    ToolchainNotInstalled { root: PathBuf },
    #[error("The TeX Live installation step failed.")]
    InstallFailed { step: String, detail: String },
    #[error("The TeX Live installer could not be downloaded.")]
    Download(#[from] DownloadError),
    #[error("No TeX Live package provides the missing files.")]
    PackagesUnavailable { files: Vec<String> },
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
