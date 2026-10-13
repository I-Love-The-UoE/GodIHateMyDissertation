use std::fmt;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PathError {
    #[error("The path escapes the project root.")]
    EscapesRoot { path: String },
    #[error("The path is empty.")]
    Empty,
    #[error("The path could not be resolved.")]
    Unresolvable {
        path: PathBuf,
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: std::io::Error,
    },
}

/// A project-relative path with forward slashes and no `..` segments.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RelPath(String);

impl RelPath {
    pub fn parse(text: &str) -> Result<Self, PathError> {
        let trimmed = text.trim().replace('\\', "/");
        let segments: Vec<&str> = trimmed
            .split('/')
            .filter(|segment| !segment.is_empty() && *segment != ".")
            .collect();

        if segments.is_empty() {
            return Err(PathError::Empty);
        }

        if segments.contains(&"..") {
            return Err(PathError::EscapesRoot {
                path: text.to_owned(),
            });
        }

        Ok(Self(segments.join("/")))
    }

    pub fn from_relative(path: &Path) -> Result<Self, PathError> {
        let mut segments = Vec::new();

        for component in path.components() {
            match component {
                Component::Normal(segment) => segments.push(segment.to_string_lossy().into_owned()),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(PathError::EscapesRoot {
                        path: path.display().to_string(),
                    });
                }
            }
        }

        if segments.is_empty() {
            return Err(PathError::Empty);
        }

        Ok(Self(segments.join("/")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn join(&self, root: &Path) -> PathBuf {
        let mut joined = root.to_path_buf();

        for segment in self.0.split('/') {
            joined.push(segment);
        }

        joined
    }

    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }

    pub fn parent(&self) -> Option<Self> {
        let (parent, _) = self.0.rsplit_once('/')?;

        Some(Self(parent.to_owned()))
    }

    pub fn child(&self, name: &str) -> Result<Self, PathError> {
        Self::parse(&format!("{}/{name}", self.0))
    }

    pub fn extension(&self) -> Option<&str> {
        let name = self.file_name();
        let (_, extension) = name.rsplit_once('.')?;

        Some(extension)
    }

    pub fn starts_with(&self, prefix: &Self) -> bool {
        self.0 == prefix.0 || self.0.starts_with(&format!("{}/", prefix.0))
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for RelPath {
    type Error = PathError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<RelPath> for String {
    fn from(value: RelPath) -> Self {
        value.0
    }
}

/// Canonicalises without producing Windows `\\?\` prefixes, which break TeX and libgit2.
pub fn normalise(path: &Path) -> Result<PathBuf, PathError> {
    dunce::canonicalize(path).map_err(|source| PathError::Unresolvable {
        path: path.to_path_buf(),
        source,
    })
}
