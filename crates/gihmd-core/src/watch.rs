use std::path::Path;

use serde::Serialize;
use tokio::sync::mpsc;

use crate::paths::RelPath;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WatchError {
    #[error("The project directory could not be watched.")]
    Unavailable { detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FsEvent {
    Created { path: RelPath },
    Modified { path: RelPath },
    Removed { path: RelPath },
    Renamed { from: RelPath, to: RelPath },
    Rescan,
}

pub trait Watcher: Send + Sync + 'static {
    /// Dropping the handle stops the watch.
    type Handle: Send + 'static;

    fn start(
        &self,
        root: &Path,
        events: mpsc::Sender<Vec<FsEvent>>,
    ) -> Result<Self::Handle, WatchError>;
}
