use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::paths::RelPath;
use crate::project::FileNode;
use crate::project::NodeKind;
use crate::project::ProjectError;
use crate::project::list_tree;

static LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\label\{([^}]+)\}").unwrap_or_else(|_| never()));
static NEW_COMMAND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\(?:re)?newcommand\*?\s*\{?\\([A-Za-z@]+)\}?").unwrap_or_else(|_| never())
});
static NEW_ENVIRONMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\(?:re)?newenvironment\*?\s*\{([^}]+)\}").unwrap_or_else(|_| never())
});
static BIB_ENTRY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@[A-Za-z]+\s*\{\s*([^,\s]+)\s*,").unwrap_or_else(|_| never()));
static USE_PACKAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\\(?P<command>usepackage|RequirePackage|documentclass|LoadClass)\s*(?:\[[^\]]*\])?\s*\{(?P<names>[^}]+)\}",
    )
    .unwrap_or_else(|_| never())
});
static COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|[^\\])%.*$").unwrap_or_else(|_| never()));

#[expect(
    clippy::panic,
    reason = "A constant regex that fails to compile is a programming error."
)]
fn never() -> Regex {
    panic!("Constant regex failed to compile.")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Symbol {
    pub name: String,
    pub file: RelPath,
    pub line: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIndex {
    pub labels: Vec<Symbol>,
    pub citations: Vec<Symbol>,
    pub commands: Vec<Symbol>,
    pub environments: Vec<Symbol>,
    pub tex_files: Vec<RelPath>,
    pub bib_files: Vec<RelPath>,
    /// Names from `\usepackage` and `\RequirePackage`, as written, in first-seen order.
    pub packages: Vec<String>,
    /// Names from `\documentclass` and `\LoadClass`.
    pub classes: Vec<String>,
    /// File names of `.sty` and `.cls` files that live inside the project.
    pub local_styles: Vec<String>,
}

fn collect_files(node: &FileNode, into: &mut Vec<RelPath>) {
    for child in &node.children {
        match child.kind {
            NodeKind::Dir => collect_files(child, into),
            NodeKind::File => {
                if let Some(path) = &child.path
                    && matches!(path.extension(), Some("tex" | "bib" | "sty" | "cls"))
                {
                    into.push(path.clone());
                }
            }
        }
    }
}

fn strip_comment(line: &str) -> String {
    COMMENT.replace(line, "$1").into_owned()
}

fn scan_tex(text: &str, file: &RelPath, index: &mut ProjectIndex) {
    for (number, raw) in text.lines().enumerate() {
        let line = strip_comment(raw);
        let line_number = u32::try_from(number + 1).unwrap_or(u32::MAX);

        for capture in LABEL.captures_iter(&line) {
            if let Some(name) = capture.get(1) {
                index.labels.push(Symbol {
                    name: name.as_str().trim().to_owned(),
                    file: file.clone(),
                    line: line_number,
                });
            }
        }

        for capture in NEW_COMMAND.captures_iter(&line) {
            if let Some(name) = capture.get(1) {
                index.commands.push(Symbol {
                    name: name.as_str().to_owned(),
                    file: file.clone(),
                    line: line_number,
                });
            }
        }

        for capture in USE_PACKAGE.captures_iter(&line) {
            let Some(names) = capture.name("names") else {
                continue;
            };
            let is_class = capture
                .name("command")
                .is_some_and(|m| m.as_str() == "documentclass" || m.as_str() == "LoadClass");
            let target = if is_class {
                &mut index.classes
            } else {
                &mut index.packages
            };

            for name in names.as_str().split(',') {
                let name = name.trim().to_owned();

                if !name.is_empty() && !target.contains(&name) {
                    target.push(name);
                }
            }
        }

        for capture in NEW_ENVIRONMENT.captures_iter(&line) {
            if let Some(name) = capture.get(1) {
                index.environments.push(Symbol {
                    name: name.as_str().to_owned(),
                    file: file.clone(),
                    line: line_number,
                });
            }
        }
    }
}

fn scan_bib(text: &str, file: &RelPath, index: &mut ProjectIndex) {
    for (number, line) in text.lines().enumerate() {
        let line_number = u32::try_from(number + 1).unwrap_or(u32::MAX);

        for capture in BIB_ENTRY.captures_iter(line) {
            if let Some(key) = capture.get(1) {
                index.citations.push(Symbol {
                    name: key.as_str().to_owned(),
                    file: file.clone(),
                    line: line_number,
                });
            }
        }
    }
}

pub async fn build_index(root: &Path) -> Result<ProjectIndex, ProjectError> {
    let tree = list_tree(root).await?;
    let mut files = Vec::new();
    collect_files(&tree, &mut files);
    let mut index = ProjectIndex::default();

    for file in files {
        let Ok(text) = tokio::fs::read_to_string(file.join(root)).await else {
            continue;
        };

        if matches!(file.extension(), Some("sty" | "cls")) {
            index.local_styles.push(file.file_name().to_owned());
        }

        if file.extension() == Some("bib") {
            scan_bib(&text, &file, &mut index);
            index.bib_files.push(file);
        } else {
            scan_tex(&text, &file, &mut index);
            index.tex_files.push(file);
        }
    }

    Ok(index)
}
