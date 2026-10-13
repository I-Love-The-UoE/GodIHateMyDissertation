use std::path::Path;

use serde::Serialize;

use crate::paths::RelPath;
use crate::project::ProjectError;

const IGNORED_NAMES: &[&str] = &[".git", "node_modules", ".gihmd", "__pycache__", ".DS_Store"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    File,
    Dir,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub path: Option<RelPath>,
    pub name: String,
    pub kind: NodeKind,
    pub children: Vec<FileNode>,
}

pub fn is_ignored_name(name: &str) -> bool {
    IGNORED_NAMES.contains(&name)
}

pub async fn list_tree(root: &Path) -> Result<FileNode, ProjectError> {
    if !root.is_dir() {
        return Err(ProjectError::RootMissing {
            root: root.to_path_buf(),
        });
    }

    let name = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let children = list_children(root, None).await?;

    Ok(FileNode {
        path: None,
        name,
        kind: NodeKind::Dir,
        children,
    })
}

async fn list_children(
    root: &Path,
    parent: Option<&RelPath>,
) -> Result<Vec<FileNode>, ProjectError> {
    let dir = parent.map_or_else(|| root.to_path_buf(), |parent| parent.join(root));
    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .map_err(|source| ProjectError::Listing {
            root: dir.clone(),
            source,
        })?;
    let mut nodes = Vec::new();

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|source| ProjectError::Listing {
            root: dir.clone(),
            source,
        })?
    {
        let name = entry.file_name().to_string_lossy().into_owned();

        if is_ignored_name(&name) {
            continue;
        }

        let path = match parent {
            Some(parent) => parent.child(&name)?,
            None => RelPath::parse(&name)?,
        };
        let file_type = entry
            .file_type()
            .await
            .map_err(|source| ProjectError::Listing {
                root: dir.clone(),
                source,
            })?;

        if file_type.is_dir() {
            let children = Box::pin(list_children(root, Some(&path))).await?;
            nodes.push(FileNode {
                path: Some(path),
                name,
                kind: NodeKind::Dir,
                children,
            });
        } else {
            nodes.push(FileNode {
                path: Some(path),
                name,
                kind: NodeKind::File,
                children: Vec::new(),
            });
        }
    }

    nodes.sort_by(|a, b| {
        (b.kind == NodeKind::Dir)
            .cmp(&(a.kind == NodeKind::Dir))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(nodes)
}
