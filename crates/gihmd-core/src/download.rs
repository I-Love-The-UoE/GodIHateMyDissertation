use std::future::Future;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DownloadError {
    #[error("The download could not be started.")]
    Request { url: String, detail: String },
    #[error("The server answered with an error status.")]
    Status { url: String, status: u16 },
    #[error("The download was interrupted.")]
    Interrupted { url: String, detail: String },
    #[error("The downloaded file could not be written.")]
    Io {
        path: PathBuf,
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub received: u64,
    pub total: Option<u64>,
}

pub trait Downloads: Send + Sync + 'static {
    fn download(
        &self,
        url: &str,
        dest: &Path,
        progress: mpsc::Sender<DownloadProgress>,
    ) -> impl Future<Output = Result<(), DownloadError>> + Send;
}
