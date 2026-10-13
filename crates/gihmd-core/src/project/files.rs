use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::paths::RelPath;
use crate::project::ProjectError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    pub mtime_ms: u64,
    pub size: u64,
}

fn io_error(path: &RelPath, source: std::io::Error) -> ProjectError {
    if source.kind() == std::io::ErrorKind::NotFound {
        return ProjectError::NotFound { path: path.clone() };
    }

    ProjectError::Io {
        path: path.clone(),
        source,
    }
}

pub async fn stat_path(root: &Path, path: &RelPath) -> Result<FileStat, ProjectError> {
    let metadata = tokio::fs::metadata(path.join(root))
        .await
        .map_err(|source| io_error(path, source))?;
    let mtime = metadata
        .modified()
        .map_err(|source| io_error(path, source))?;
    let mtime_ms = mtime.duration_since(UNIX_EPOCH).map_or(0, |duration| {
        u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
    });

    Ok(FileStat {
        mtime_ms,
        size: metadata.len(),
    })
}

pub async fn read_text(root: &Path, path: &RelPath) -> Result<(String, FileStat), ProjectError> {
    let bytes = tokio::fs::read(path.join(root))
        .await
        .map_err(|source| io_error(path, source))?;
    let text = String::from_utf8(bytes).map_err(|source| ProjectError::NotUtf8 {
        path: path.clone(),
        source,
    })?;
    let stat = stat_path(root, path).await?;

    Ok((text, stat))
}

/// Writes atomically (temporary file + rename). When `expected_mtime_ms` is given and the file
/// has changed since, nothing is written and `StaleWrite` is returned.
pub async fn write_text(
    root: &Path,
    path: &RelPath,
    text: &str,
    expected_mtime_ms: Option<u64>,
) -> Result<FileStat, ProjectError> {
    if let Some(expected) = expected_mtime_ms {
        let current = stat_path(root, path).await?;

        if current.mtime_ms != expected {
            return Err(ProjectError::StaleWrite {
                path: path.clone(),
                mtime_ms: current.mtime_ms,
            });
        }
    }

    let target = path.join(root);

    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|source| io_error(path, source))?;
    }

    let temporary = target.with_extension(format!("{}.gihmd-tmp", path.extension().unwrap_or("")));
    tokio::fs::write(&temporary, text)
        .await
        .map_err(|source| io_error(path, source))?;
    tokio::fs::rename(&temporary, &target)
        .await
        .map_err(|source| io_error(path, source))?;

    stat_path(root, path).await
}

pub async fn create_file(root: &Path, path: &RelPath) -> Result<(), ProjectError> {
    let target = path.join(root);

    if tokio::fs::try_exists(&target)
        .await
        .map_err(|source| io_error(path, source))?
    {
        return Err(ProjectError::AlreadyExists { path: path.clone() });
    }

    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|source| io_error(path, source))?;
    }

    tokio::fs::write(&target, "")
        .await
        .map_err(|source| io_error(path, source))
}

pub async fn create_dir(root: &Path, path: &RelPath) -> Result<(), ProjectError> {
    let target = path.join(root);

    if tokio::fs::try_exists(&target)
        .await
        .map_err(|source| io_error(path, source))?
    {
        return Err(ProjectError::AlreadyExists { path: path.clone() });
    }

    tokio::fs::create_dir_all(&target)
        .await
        .map_err(|source| io_error(path, source))
}

pub async fn rename_path(root: &Path, from: &RelPath, to: &RelPath) -> Result<(), ProjectError> {
    let target = to.join(root);

    if tokio::fs::try_exists(&target)
        .await
        .map_err(|source| io_error(to, source))?
    {
        return Err(ProjectError::AlreadyExists { path: to.clone() });
    }

    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|source| io_error(to, source))?;
    }

    tokio::fs::rename(from.join(root), target)
        .await
        .map_err(|source| io_error(from, source))
}

pub async fn move_path(
    root: &Path,
    from: &RelPath,
    to_dir: Option<&RelPath>,
) -> Result<RelPath, ProjectError> {
    let to = match to_dir {
        Some(dir) => dir.child(from.file_name())?,
        None => RelPath::parse(from.file_name())?,
    };
    rename_path(root, from, &to).await?;

    Ok(to)
}

pub async fn delete_path(root: &Path, path: &RelPath) -> Result<(), ProjectError> {
    let target = path.join(root);
    let metadata = tokio::fs::metadata(&target)
        .await
        .map_err(|source| io_error(path, source))?;

    if metadata.is_dir() {
        return tokio::fs::remove_dir_all(&target)
            .await
            .map_err(|source| io_error(path, source));
    }

    tokio::fs::remove_file(&target)
        .await
        .map_err(|source| io_error(path, source))
}
