use std::path::Path;

use serde::Serialize;

use crate::paths::RelPath;
use crate::project::FileNode;
use crate::project::NodeKind;
use crate::project::ProjectError;
use crate::project::list_tree;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MainFileDetection {
    Exactly { path: RelPath },
    Candidates { paths: Vec<RelPath> },
    None,
}

fn collect_tex_files(node: &FileNode, into: &mut Vec<RelPath>) {
    for child in &node.children {
        match child.kind {
            NodeKind::Dir => collect_tex_files(child, into),
            NodeKind::File => {
                if let Some(path) = &child.path
                    && path.extension() == Some("tex")
                {
                    into.push(path.clone());
                }
            }
        }
    }
}

/// A main file is a `.tex` file containing `\documentclass`; the search prefers the project root.
pub async fn detect_main_file(root: &Path) -> Result<MainFileDetection, ProjectError> {
    let tree = list_tree(root).await?;
    let mut tex_files = Vec::new();
    collect_tex_files(&tree, &mut tex_files);
    let mut candidates = Vec::new();

    for path in tex_files {
        let Ok(text) = tokio::fs::read_to_string(path.join(root)).await else {
            continue;
        };

        if text.contains("\\documentclass") {
            candidates.push(path);
        }
    }

    candidates.sort_by_key(|path| (path.as_str().matches('/').count(), path.as_str().to_owned()));

    match candidates.len() {
        0 => Ok(MainFileDetection::None),
        1 => Ok(MainFileDetection::Exactly {
            path: candidates.remove(0),
        }),
        _ => {
            if let Some(main) = candidates.iter().find(|path| path.as_str() == "main.tex") {
                return Ok(MainFileDetection::Exactly { path: main.clone() });
            }

            Ok(MainFileDetection::Candidates { paths: candidates })
        }
    }
}
