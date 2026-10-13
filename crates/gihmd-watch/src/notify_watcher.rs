use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use gihmd_core::FsEvent;
use gihmd_core::RelPath;
use gihmd_core::WatchError;
use gihmd_core::Watcher;
use gihmd_core::project::is_ignored_name;
use notify::EventKind;
use notify::RecursiveMode;
use notify::event::ModifyKind;
use notify::event::RenameMode;
use notify_debouncer_full::DebounceEventResult;
use notify_debouncer_full::Debouncer;
use notify_debouncer_full::RecommendedCache;
use notify_debouncer_full::new_debouncer;
use tokio::sync::mpsc;

const DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Default)]
pub struct NotifyWatcher;

/// Keeps the debouncer alive; dropping it stops the watch.
#[derive(Debug)]
pub struct WatchHandle {
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
}

fn relative(root: &Path, path: &Path) -> Option<RelPath> {
    let stripped = path.strip_prefix(root).ok()?;

    if stripped
        .components()
        .any(|component| is_ignored_name(&component.as_os_str().to_string_lossy()))
    {
        return None;
    }

    RelPath::from_relative(stripped).ok()
}

fn translate(root: &Path, event: &notify::Event) -> Vec<FsEvent> {
    let paths: Vec<&PathBuf> = event.paths.iter().collect();

    match event.kind {
        EventKind::Create(_) => paths
            .iter()
            .filter_map(|p| relative(root, p))
            .map(|path| FsEvent::Created { path })
            .collect(),
        EventKind::Remove(_) => paths
            .iter()
            .filter_map(|p| relative(root, p))
            .map(|path| FsEvent::Removed { path })
            .collect(),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            let from = paths.first().and_then(|p| relative(root, p));
            let to = paths.get(1).and_then(|p| relative(root, p));

            match (from, to) {
                (Some(from), Some(to)) => vec![FsEvent::Renamed { from, to }],
                (Some(from), None) => vec![FsEvent::Removed { path: from }],
                (None, Some(to)) => vec![FsEvent::Created { path: to }],
                (None, None) => Vec::new(),
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => paths
            .iter()
            .filter_map(|p| relative(root, p))
            .map(|path| FsEvent::Removed { path })
            .collect(),
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => paths
            .iter()
            .filter_map(|p| relative(root, p))
            .map(|path| FsEvent::Created { path })
            .collect(),
        EventKind::Modify(_) => paths
            .iter()
            .filter_map(|p| relative(root, p))
            .map(|path| FsEvent::Modified { path })
            .collect(),
        EventKind::Other | EventKind::Any | EventKind::Access(_) => Vec::new(),
    }
}

impl Watcher for NotifyWatcher {
    type Handle = WatchHandle;

    fn start(
        &self,
        root: &Path,
        events: mpsc::Sender<Vec<FsEvent>>,
    ) -> Result<Self::Handle, WatchError> {
        let root_owned = root.to_path_buf();
        let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
            let batch: Vec<FsEvent> = match result {
                Ok(debounced) => debounced
                    .iter()
                    .flat_map(|event| translate(&root_owned, &event.event))
                    .collect(),
                Err(errors) => {
                    for error in errors {
                        tracing::warn!(error = %error, "The file watcher reported an error.");
                    }

                    vec![FsEvent::Rescan]
                }
            };

            if !batch.is_empty() && events.blocking_send(batch).is_err() {
                tracing::debug!("The file watcher receiver is gone.");
            }
        })
        .map_err(|error| WatchError::Unavailable {
            detail: error.to_string(),
        })?;
        debouncer
            .watch(root, RecursiveMode::Recursive)
            .map_err(|error| WatchError::Unavailable {
                detail: error.to_string(),
            })?;

        Ok(WatchHandle {
            _debouncer: debouncer,
        })
    }
}

pub fn default() -> NotifyWatcher {
    NotifyWatcher
}
